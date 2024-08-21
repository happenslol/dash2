#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;
use gdk::Monitor;
use gtk::traits::WidgetExt;
use gtk_layer_shell::LayerShell;
use serde::Deserialize;
use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  net::UnixStream,
};
use webkit2gtk::WebViewExt;
use tauri::{Manager, Emitter};

#[tokio::main]
async fn main() {
  tauri::async_runtime::set(tokio::runtime::Handle::current());

  let app = tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![quit])
    .build(tauri::generate_context!())
    .expect("error while building tauri application");

  let display = gdk::Display::default().unwrap();
  let n_monitors = display.n_monitors();

  let hl_monitors = get_hyprland_monitors().await;

  let mut greeter_windows: Vec<tauri::WebviewWindow> = Vec::with_capacity(n_monitors as usize);
  for i in 0..n_monitors {
    let monitor = display.monitor(i).unwrap();
    let hl_monitor = hl_monitors.iter().find(|m| m.id == i).unwrap();
    let window = create_overlay_window(
      &app,
      "test",
      "src/login/index.html",
      &monitor,
      gtk_layer_shell::Layer::Top,
      (true, true, true, true),
      (0, 0),
    )
    .expect("failed to create window");

    window.emit("test", "test").unwrap();
    greeter_windows.push(window);
  }

  app.run(|_, _| {});
}

#[tauri::command]
async fn quit(app: tauri::AppHandle, window: tauri::Window) {
  app.exit(0);
}

#[derive(Debug, Deserialize)]
struct HLMonitor {
  id: i32,
  name: String,
  model: String,
  width: i32,
  height: i32,
  disabled: bool,
}

async fn get_hyprland_monitors() -> Vec<HLMonitor> {
  let his = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").unwrap();
  let rt_dir = std::env::var("XDG_RUNTIME_DIR").unwrap();
  let mut socket = UnixStream::connect(format!("{rt_dir}/hypr/{his}/.socket.sock"))
    .await
    .unwrap();

  socket.write_all(b"j/monitors").await.unwrap();
  let mut res = String::new();
  socket.read_to_string(&mut res).await.unwrap();

  serde_json::from_str::<Vec<HLMonitor>>(&res).unwrap()
}

fn create_overlay_window(
  app: &tauri::App,
  label: &str,
  url: &str,
  monitor: &Monitor,
  layer: gtk_layer_shell::Layer,
  edge: (bool, bool, bool, bool),
  size: (i32, i32),
) -> Result<tauri::WebviewWindow> {
  let window = tauri::WebviewWindow::builder(app, label, tauri::WebviewUrl::App(url.into()))
    .resizable(false)
    .decorations(false)
    .visible(false)
    .build()?;

  let gtk_window = window.gtk_window()?;
  gtk_window.init_layer_shell();
  gtk_window.set_monitor(monitor);
  gtk_window.set_layer(layer);
  gtk_window.set_keyboard_interactivity(true);

  let (top, right, bottom, left) = edge;
  gtk_window.set_anchor(gtk_layer_shell::Edge::Top, top);
  gtk_window.set_anchor(gtk_layer_shell::Edge::Right, right);
  gtk_window.set_anchor(gtk_layer_shell::Edge::Bottom, bottom);
  gtk_window.set_anchor(gtk_layer_shell::Edge::Left, left);

  let (width, height) = size;
  gtk_window.set_size_request(width, height);
  gtk_window.set_app_paintable(true);

  window.with_webview(|webview| {
    let webview = webview.inner();
    webview.set_background_color(&gdk::RGBA::new(0., 0., 0., 0.));
  })?;

  window.show()?;

  Ok(window)
}

// fn main() {
//   let app = tauri::Builder::default()
//     .build(tauri::generate_context!())
//     .expect("error while building tauri application");
//
//   let display = gdk::Display::default().unwrap();
//
//   let init_window = tauri::WindowBuilder::new(&app, "init", tauri::WindowUrl::App("".into()))
//     .visible(false)
//     .build()
//     .expect("failed to create init window");
//
//   let gtk_app = init_window
//     .gtk_window()
//     .expect("failed to get window for init app")
//     .application()
//     .expect("failed to get application from init window");
//
//   init_window.close().unwrap();
//
//   let main_monitor = display.monitor(0).unwrap();
//   let top_window = gtk::ApplicationWindow::new(&gtk_app);
//
//   gtk_layer_shell::init_for_window(&top_window);
//   gtk_layer_shell::set_monitor(&top_window, &main_monitor);
//   gtk_layer_shell::set_layer(&top_window, gtk_layer_shell::Layer::Top);
//
//   gtk_layer_shell::set_anchor(&top_window, gtk_layer_shell::Edge::Top, true);
//   gtk_layer_shell::set_anchor(&top_window, gtk_layer_shell::Edge::Right, true);
//   gtk_layer_shell::set_anchor(&top_window, gtk_layer_shell::Edge::Left, true);
//
//   top_window.set_resizable(false);
//   top_window.set_decorated(false);
//   let width = main_monitor.geometry().width();
//   top_window.set_size_request(width, 15);
//   top_window.set_app_paintable(true);
//
//   top_window.show_all();
//
//   create_overlay_window(
//     &app,
//     "test",
//     "test.html",
//     &main_monitor,
//     gtk_layer_shell::Layer::Top,
//     (false, false, false, false),
//     (400, 400),
//   )
//   .expect("failed to create test window");
//
//   top_window.connect_enter_notify_event(|_window, _ev| {
//     println!("entered top window");
//     gtk::Inhibit(true)
//   });
//
//   app.run(|_, _| {});
// }
//
// fn create_overlay_window(
//   app: &tauri::App,
//   label: &str,
//   url: &str,
//   monitor: &Monitor,
//   layer: gtk_layer_shell::Layer,
//   edge: (bool, bool, bool, bool),
//   size: (i32, i32),
// ) -> Result<tauri::Window> {
//   let window = tauri::WindowBuilder::new(app, label, tauri::WindowUrl::App(url.into()))
//     .resizable(false)
//     .decorations(false)
//     .visible(false)
//     .build()?;
//
//   let gtk_window = window.gtk_window()?;
//   gtk_layer_shell::init_for_window(&gtk_window);
//   gtk_layer_shell::set_monitor(&gtk_window, monitor);
//   gtk_layer_shell::set_layer(&gtk_window, layer);
//
//   let (top, right, bottom, left) = edge;
//
//   if top {
//     gtk_layer_shell::set_anchor(&gtk_window, gtk_layer_shell::Edge::Top, true);
//   }
//   if right {
//     gtk_layer_shell::set_anchor(&gtk_window, gtk_layer_shell::Edge::Right, true);
//   }
//   if bottom {
//     gtk_layer_shell::set_anchor(&gtk_window, gtk_layer_shell::Edge::Bottom, true);
//   }
//   if left {
//     gtk_layer_shell::set_anchor(&gtk_window, gtk_layer_shell::Edge::Left, true);
//   }
//
//   let (width, height) = size;
//   gtk_window.set_size_request(width, height);
//   gtk_window.set_app_paintable(true);
//
//   gtk_window.connect_enter_notify_event(|_window, _ev| {
//     println!("entered");
//     gtk::Inhibit(true)
//   });
//
//   gtk_window.connect_leave_notify_event(|_window, _ev| {
//     println!("left");
//     gtk::Inhibit(true)
//   });
//
//   let window_children = gtk_window.children();
//   let root_box = window_children[0].downcast_ref::<gtk::Box>().unwrap();
//   let root_box_children = root_box.children();
//   let menu_bar = root_box_children[0].downcast_ref::<gtk::MenuBar>().unwrap();
//   root_box.remove(menu_bar);
//
//   window.with_webview(|webview| {
//     let webview = webview.inner();
//     webview.set_background_color(&gdk::RGBA::new(0., 0., 0., 0.));
//   })?;
//
//   window.show()?;
//
//   Ok(window)
// }
