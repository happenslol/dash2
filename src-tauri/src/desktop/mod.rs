use crate::{
  battery::BatterySubscription,
  config::Config,
  hyprland::{events::Event, HyprlandClient, HyprlandListener},
  layer_shell::LayerShellWindowBuilder,
  power::Power,
};
use anyhow::Result;
use futures::StreamExt;
use gdk::{
  cairo::{
    ffi::{
      cairo_rectangle, cairo_region_create, cairo_region_create_rectangle, cairo_region_destroy,
      cairo_region_union, cairo_region_union_rectangle,
    },
    RectangleInt, Region,
  },
  Monitor,
};
use gtk::prelude::*;
use tauri::{Emitter, Manager};
use tracing::error;

const NAMESPACE: &str = "dash2-desktop";

struct TauriState<'a> {
  config: Config,
  battery: BatterySubscription<'a>,
  hyprland: HyprlandClient,
  power: Power,
  desktop_windows: Vec<tauri::WebviewWindow>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct InputRegion {
  x: i32,
  y: i32,
  width: i32,
  height: i32,
}

pub fn run(config: Config) -> Result<()> {
  let rt = tokio::runtime::Runtime::new()?;
  rt.block_on(async {
    tauri::async_runtime::set(tokio::runtime::Handle::current());

    let app = tauri::Builder::default()
      .invoke_handler(tauri::generate_handler![
        log,
        window_ready,
        request_input_regions,
      ])
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
          }
          event => println!("event: {event:?}"),
        }
      }
    });

    let hyprland_client = HyprlandClient::new().await?;

    // TODO: Handle monitor changes
    let display = gdk::Display::default().unwrap();
    let mut desktop_windows = Vec::with_capacity(display.n_monitors() as usize);

    for n in 0..display.n_monitors() {
      let monitor = display.monitor(n).unwrap();
      desktop_windows.push(create_monitor_window(app.handle(), n, &monitor)?);
    }

    app.manage(TauriState {
      config: config.clone(),
      hyprland: hyprland_client,
      battery,
      power,
      desktop_windows,
    });

    app.run(|_, _| {});

    Ok(())
  })
}

async fn assign_primary(app: &tauri::AppHandle) -> Result<()> {
  let state = app.state::<TauriState>();
  let hyprland_monitors = state.hyprland.get_monitors().await?;

  let app_handle = app.clone();
  app
    .run_on_main_thread(move || {
      let state = app_handle.state::<TauriState>();
      let primary_index = state
        .config
        .primary_display
        .iter()
        .find_map(|name| {
          hyprland_monitors
            .iter()
            .position(|monitor| &monitor.name == name)
        })
        .unwrap_or(0);

      let display = gdk::Display::default().unwrap();
      for n in 0..display.n_monitors() {
        let window = state.desktop_windows[n as usize].clone();

        if n == primary_index as i32 {
          window.emit_to(window.label(), "is-primary", true).unwrap();
        } else {
          window.emit_to(window.label(), "is-primary", false).unwrap();
        }
      }
    })
    .unwrap();

  Ok(())
}

fn create_monitor_window(
  app: &tauri::AppHandle,
  index: i32,
  monitor: &Monitor,
) -> Result<tauri::WebviewWindow> {
  let label = format!("{NAMESPACE}-{index}");
  let window = LayerShellWindowBuilder::new(&label, "src/desktop/index.html")
    .layer(gtk_layer_shell::Layer::Top)
    .monitor(monitor)
    .keyboard_mode(gtk_layer_shell::KeyboardMode::OnDemand)
    .namespace(NAMESPACE)
    .edge(true, true, true, true)
    .size(0, 0)
    .background_color(0., 0., 0., 0.)
    .build(app)?;

  if index == 0 {
    #[cfg(debug_assertions)]
    window.open_devtools();
  }

  let gtk_window = window.gtk_window()?;
  let empty_region = Region::create_rectangle(&RectangleInt::new(0, 0, 0, 0));
  gtk_window.input_shape_combine_region(Some(&empty_region));

  let window_handle = window.clone();
  gtk_window.connect_enter_notify_event(move |_, event| {
    println!("enter at {:?}", event.position());
    window_handle
      .emit_to(window_handle.label(), "enter", event.position())
      .unwrap_or_else(|err| {
        error!("failed to emit enter: {err}");
      });

    gdk::glib::Propagation::Stop
  });

  let window_handle = window.clone();
  gtk_window.connect_leave_notify_event(move |_, event| {
    println!("leave at {:?}", event.position());
    window_handle
      .emit_to(window_handle.label(), "leave", event.position())
      .unwrap_or_else(|err| {
        error!("failed to emit leave: {err}");
      });

    gdk::glib::Propagation::Stop
  });

  Ok(window)
}

#[tauri::command]
async fn log(window: tauri::WebviewWindow, message: String) {
  println!("{}: {}", window.label(), message);
}

#[tauri::command]
async fn window_ready(app: tauri::AppHandle) {
  assign_primary(&app).await.unwrap();
}

#[tauri::command]
async fn request_input_regions(window: tauri::WebviewWindow, regions: Vec<InputRegion>) {
  let gtk_window = match window.gtk_window() {
    Ok(window) => window,
    Err(err) => {
      error!("failed to get gtk window: {err}");
      return;
    }
  };

  let region = Region::create_rectangles(&regions.iter().map(|region| {
    RectangleInt::new(region.x, region.y, region.width, region.height)
  }).collect::<Vec<_>>());
  gtk_window.input_shape_combine_region(Some(&region));

  // unsafe {
  //   let combined = cairo_region_create();
  //   for region in regions {
  //     let mut rect = gdk::cairo::ffi::cairo_rectangle_int_t {
  //       x: region.x,
  //       y: region.y,
  //       width: region.width,
  //       height: region.height,
  //     };
  //
  //     let region = cairo_region_create_rectangle(&mut rect as *mut _);
  //     cairo_region_union(combined, region);
  //     cairo_region_destroy(region);
  //   }
  //
  //   let borrowed = Region::from_raw_borrow(combined);
  //   gtk_window.input_shape_combine_region(Some(&borrowed));
  //   cairo_region_destroy(combined);
  // }
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
