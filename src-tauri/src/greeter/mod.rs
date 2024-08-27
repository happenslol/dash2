use anyhow::Result;
use gdk::prelude::*;
use tauri::{Emitter, Manager};
use tokio::sync::Mutex;

use crate::{
  battery::{BatteryState, BatterySubscription},
  config::Config,
  hyprland::HyprlandConn,
  layer_shell::LayerShellWindowBuilder,
  power::Power,
  util::rand_string,
};

use self::greetd::GreetdClient;

mod greetd;

struct TauriState<'a> {
  config: Config,
  greetd: Option<Mutex<GreetdClient>>,
  battery: BatterySubscription<'a>,
  hyprland: HyprlandConn,
  power: Power,
}

pub fn greet(config: Config, demo: bool) -> Result<()> {
  let rt = tokio::runtime::Runtime::new()?;
  rt.block_on(async {
    tauri::async_runtime::set(tokio::runtime::Handle::current());

    let app = tauri::Builder::default()
      .invoke_handler(tauri::generate_handler![
        poweroff,
        suspend,
        get_battery_state,
        window_ready,
        quit,
        submit_password,
      ])
      .build(tauri::generate_context!())?;

    let display =
      gdk::Display::default().ok_or_else(|| anyhow::anyhow!("failed to get display"))?;

    let hyprland_conn = HyprlandConn::new().await?;

    let zbus_conn = zbus::Connection::system().await?;
    let battery = BatterySubscription::new(app.handle(), &zbus_conn).await?;
    let power = Power::new(zbus_conn);

    let greetd_client = if demo {
      None
    } else {
      Some(Mutex::new(greetd::GreetdClient::new(config.clone()).await?))
    };

    app.manage(TauriState {
      config: config.clone(),
      hyprland: hyprland_conn,
      greetd: greetd_client,
      battery,
      power,
    });

    for i in 0..display.n_monitors() {
      let monitor = display.monitor(i).unwrap();
      create_greeter_window(app.handle(), &monitor)?;
    }

    let app_handle = app.handle().clone();
    display.connect_monitor_added(move |_display, monitor| {
      create_greeter_window(&app_handle, monitor).unwrap();
    });

    let app_handle = app.handle().clone();
    display.connect_monitor_removed(move |_display, monitor| {
      let config = config.clone();
      let assign_app_handle = app_handle.clone();
      tokio::spawn(async move {
        assign_primary(config.clone(), &assign_app_handle)
          .await
          .unwrap_or_else(|err| {
            eprintln!("failed to assign primary display: {err}");
          });
      });

      let Some(window_label) = (unsafe { monitor.data::<String>("window-label") }) else {
        return;
      };

      let window_label = unsafe { window_label.as_ref() }.clone();
      let Some(window) = app_handle.get_webview_window(&window_label) else {
        return;
      };

      window.close().unwrap_or_else(|err| {
        eprintln!("failed to close window: {err}");
      });
    });

    app.run(|_, _| {});
    Ok(())
  })
}

fn create_greeter_window(app: &tauri::AppHandle, monitor: &gdk::Monitor) -> Result<()> {
  let label = format!("greeter-{}", rand_string());

  LayerShellWindowBuilder::new(&label, "src/login/index.html")
    .layer(gtk_layer_shell::Layer::Top)
    .monitor(monitor)
    .keyboard_mode(gtk_layer_shell::KeyboardMode::OnDemand)
    .namespace("dash2-greeter")
    .edge(true, true, true, true)
    .size(0, 0)
    .build(app)?;

  unsafe {
    monitor.set_data("window-label", label);
  }

  Ok(())
}

async fn assign_primary(config: Config, app: &tauri::AppHandle) -> Result<()> {
  let state = app.state::<TauriState>();
  let hyprland_monitors = state.hyprland.get_monitors().await?;
  let primary_index = config
    .primary_display
    .iter()
    .find_map(|name| {
      hyprland_monitors
        .iter()
        .position(|monitor| &monitor.name == name)
    })
    .unwrap_or(0);

  let app_handle = app.clone();
  app.run_on_main_thread(move || {
    let display = gdk::Display::default().unwrap();

    for index in 0..display.n_monitors() {
      if let Some(window_label) = display
        .monitor(index)
        .and_then(|monitor| unsafe { monitor.data::<String>("window-label") })
      {
        let window_label = unsafe { window_label.as_ref() }.clone();
        app_handle
          .emit_to(&window_label, "is-primary", index == primary_index as i32)
          .unwrap();
      }
    }
  })?;

  Ok(())
}

#[tauri::command]
async fn window_ready(app: tauri::AppHandle) {
  let state = app.state::<TauriState>();
  assign_primary(state.config.clone(), &app)
    .await
    .unwrap_or_else(|err| {
      eprintln!("failed to assign primary display: {err}");
    });
}

#[tauri::command]
async fn poweroff(app: tauri::AppHandle) {
  let state = app.state::<TauriState>();
  state.power.poweroff().await.unwrap_or_else(|err| {
    eprintln!("failed to poweroff: {err}");
  });
}

#[tauri::command]
async fn reboot(app: tauri::AppHandle) {
  let state = app.state::<TauriState>();
  state.power.reboot().await.unwrap_or_else(|err| {
    eprintln!("failed to reboot: {err}");
  });
}

#[tauri::command]
async fn suspend(app: tauri::AppHandle) {
  let state = app.state::<TauriState>();
  state.power.suspend().await.unwrap_or_else(|err| {
    eprintln!("failed to suspend: {err}");
  });
}

#[tauri::command]
async fn quit(app: tauri::AppHandle) {
  app.exit(0);
}

#[tauri::command]
async fn get_battery_state(app: tauri::AppHandle) -> Option<BatteryState> {
  let state = app.state::<TauriState>();
  state.battery.get_state().await.unwrap_or(None)
}

#[tauri::command]
async fn submit_password(app: tauri::AppHandle, value: String) {
  let state = app.state::<TauriState>();

  // No greetd client means we're in demo mode
  if let Some(greetd) = &state.greetd {
    let mut greetd = greetd.lock().await;

    match greetd.authenticate(value).await {
      Ok(_) => app.exit(0),
      Err(err) => {
        eprintln!("failed to authenticate: {err}");
        app
          .emit("password-error", err.to_string())
          .unwrap_or_else(|err| {
            eprintln!("failed to emit password-error: {err}");
          });
      }
    }
  } else if value == "password" {
    app.exit(0);
  } else {
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    app
      .emit("password-error", "failed to authenticate".to_string())
      .unwrap_or_else(|err| {
        eprintln!("failed to emit password-error: {err}");
      });
  }
}
