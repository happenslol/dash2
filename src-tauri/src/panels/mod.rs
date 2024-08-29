use crate::{
  battery::BatterySubscription, config::Config, hyprland::{events::Event, HyprlandListener, HyprlandClient},
  layer_shell::LayerShellWindowBuilder, power::Power,
};
use anyhow::Result;
use futures::StreamExt;
use gdk::Monitor;
use gtk::prelude::*;
use tauri::{Emitter, Manager};
use tracing::error;

const WIDTH: i32 = 1400;
const HIDDEN_HEIGHT: i32 = 6;
const VISIBLE_HEIGHT: i32 = 200;

struct TauriState<'a> {
  config: Config,
  battery: BatterySubscription<'a>,
  hyprland: HyprlandClient,
  power: Power,
}

pub fn run(config: Config) -> Result<()> {
  let rt = tokio::runtime::Runtime::new()?;
  rt.block_on(async {
    tauri::async_runtime::set(tokio::runtime::Handle::current());

    let app = tauri::Builder::default()
      .invoke_handler(tauri::generate_handler![hide_control])
      .build(tauri::generate_context!())?;

    let zbus_conn = zbus::Connection::system().await?;
    let battery = BatterySubscription::new(app.handle(), &zbus_conn).await?;
    let power = Power::new(zbus_conn);

    tokio::spawn(async move {
      let event_listener = HyprlandListener::new().await.unwrap();
      let mut stream = event_listener.listen().await.unwrap();

      while let Some(event) = stream.next().await {
        match event {
          Event::Workspace(workspace) => {
            println!("workspace changed: {workspace:?}")
          },
          event => println!("event: {event:?}"),
        }
      }
    });

    let hyprland_client = HyprlandClient::new().await?;

    app.manage(TauriState {
      config: config.clone(),
      hyprland: hyprland_client,
      battery,
      power,
    });

    let state = app.state::<TauriState>();

    let display = gdk::Display::default().unwrap();
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

    let primary_monitor = display.monitor(primary_index as i32).unwrap();
    run_control(app.handle(), &primary_monitor)?;

    app.run(|_, _| {});

    Ok(())
  })
}

fn run_control(app: &tauri::AppHandle, monitor: &Monitor) -> Result<()> {
  let control = LayerShellWindowBuilder::new("panel-main", "src/control/index.html")
    .layer(gtk_layer_shell::Layer::Top)
    .monitor(monitor)
    .keyboard_mode(gtk_layer_shell::KeyboardMode::OnDemand)
    .namespace("dash2-control")
    .edge(false, false, true, false)
    .size(WIDTH, HIDDEN_HEIGHT)
    .background_color(0., 0., 0., 0.0)
    .build(app)?;

  #[cfg(debug_assertions)]
  control.open_devtools();

  let control_gtk = control.gtk_window()?;

  let control_handle = control.clone();
  control_gtk.connect_enter_notify_event(move |control_gtk, _| {
    control_handle.emit("enter", ()).unwrap_or_else(|err| {
      error!("failed to emit enter: {err}");
    });

    control_gtk.set_size_request(WIDTH, VISIBLE_HEIGHT);
    gdk::glib::Propagation::Stop
  });

  let control_handle = control.clone();
  control_gtk.connect_leave_notify_event(move |_, _| {
    control_handle.emit("leave", ()).unwrap_or_else(|err| {
      error!("failed to emit leave: {err}");
    });

    gdk::glib::Propagation::Stop
  });

  Ok(())
}

#[tauri::command]
async fn hide_control(window: tauri::WebviewWindow) {
  let gtk_window = window.gtk_window().unwrap();
  gtk_window.set_size_request(WIDTH, HIDDEN_HEIGHT);
}

// let control_gtk = control.gtk_window().unwrap();
//
// let control_is_visible = Arc::new(AtomicBool::new(false));
// let control_is_hiding = Arc::new(AtomicBool::new(false));
//
// let (control_hide_tx, mut control_hide_rx) = channel::<()>(1);
// let (control_hide_cancel_tx, mut control_hide_cancel_rx) = channel::<()>(1);
//
// let (control_should_hide_tx, mut control_should_hide_rx) = channel::<()>(1);
//
// let control_is_visible_move = control_is_visible.clone();
// let control_is_hiding_move = control_is_hiding.clone();
// tokio::spawn(async move {
//   loop {
//     control_hide_rx.recv().await.unwrap();
//     control_is_hiding_move.store(true, Ordering::Relaxed);
//
//     tokio::select! {
//       _ = control_hide_cancel_rx.recv() => {
//         control_is_hiding_move.store(false, Ordering::Relaxed);
//         continue;
//       },
//       _ = tokio::time::sleep(Duration::from_secs(2)) => {
//         control_is_visible_move.store(false, Ordering::Relaxed);
//         control_is_hiding_move.store(false, Ordering::Relaxed);
//         control_should_hide_tx.send(()).unwrap();
//       },
//     }
//   }
// });
//
// let control_handle = control.clone();
// let control_is_visible_move = control_is_visible.clone();
// control_gtk.connect_enter_notify_event(move |_, _| {
//   if control_is_hiding.load(Ordering::Relaxed) {
//     control_hide_cancel_tx.send(()).unwrap();
//     return gdk::glib::Propagation::Stop;
//   }
//
//   if control_is_visible_move.load(Ordering::Relaxed) {
//     return gdk::glib::Propagation::Stop;
//   }
//
//   control_is_visible_move.store(true, Ordering::Relaxed);
//   let gtk_window = control_handle.gtk_window().unwrap();
//   gtk_window.set_size_request(WIDTH, VISIBLE_HEIGHT);
//
//   gdk::glib::Propagation::Stop
// });
//
// let control_is_visible_move = control_is_visible.clone();
// control_gtk.connect_leave_notify_event(move |_, _| {
//   if control_is_visible_move.load(Ordering::Relaxed) {
//     return gdk::glib::Propagation::Stop;
//   }
//
//   gdk::glib::Propagation::Stop
// });
//
// tokio::spawn(async move {});
