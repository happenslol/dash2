#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code)]

use anyhow::Result;
use futures::stream::StreamExt;
use gdk::{prelude::*, Monitor, glib::translate::ToGlibPtr};
use gtk::traits::WidgetExt;
use gtk_layer_shell::LayerShell;
use serde::Deserialize;
use tauri::{Emitter, Listener, Manager};
use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  net::UnixStream,
};
use webkit2gtk::WebViewExt;

mod pam;
mod scrambler;

struct SessionConv(String);

impl pam::converse::Converse for SessionConv {
  fn prompt_echo(&self, _msg: &str) -> Result<String, ()> {
    Err(())
  }
  fn prompt_blind(&self, _msg: &str) -> Result<String, ()> {
    Ok(self.0.clone())
  }
  fn info(&self, msg: &str) -> Result<(), ()> {
    eprintln!("pam info: {msg}");
    Err(())
  }
  fn error(&self, msg: &str) -> Result<(), ()> {
    eprintln!("pam error: {msg}");
    Err(())
  }
}

fn get_username(uid: u32) -> Option<String> {
  let mut passwd = unsafe { std::mem::zeroed::<libc::passwd>() };
  let mut buf = vec![0; 2048];
  let mut result = std::ptr::null_mut::<libc::passwd>();

  loop {
    let r = unsafe { libc::getpwuid_r(uid, &mut passwd, buf.as_mut_ptr(), buf.len(), &mut result) };

    if r != libc::ERANGE {
      break;
    }

    let newsize = buf.len().checked_mul(2)?;
    buf.resize(newsize, 0);
  }

  if result.is_null() {
    // There is no such user, or an error has occurred.
    // errno gets set if there’s an error.
    return None;
  }

  if result != &mut passwd {
    // The result of getpwuid_r should be its input passwd.
    return None;
  }

  let raw = unsafe { std::ffi::CStr::from_ptr(result.read().pw_name) };
  Some(String::from(raw.to_string_lossy()))
}

#[tokio::main]
async fn main() {
  tauri::async_runtime::set(tokio::runtime::Handle::current());

  let app = tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![quit, submit_password])
    .build(tauri::generate_context!())
    .expect("error while building tauri application");

  let connection = zbus::Connection::system().await.unwrap();
  let upower = upower_dbus::UPowerProxy::new(&connection).await.unwrap();

  let display_device = upower.get_display_device().await.unwrap();
  let ttype = display_device.type_().await.unwrap();
  let has_battery = ttype == upower_dbus::BatteryType::Battery;

  let mut percentage = 0f64;
  let mut psu_connected = false;

  if has_battery {
    percentage = display_device.percentage().await.unwrap();
    psu_connected = display_device.power_supply().await.unwrap();

    let handle = app.app_handle().clone();
    let display_device_handle = display_device.clone();
    tokio::spawn(async move {
      let mut stream = display_device_handle.receive_percentage_changed().await;
      while let Some(ev) = stream.next().await {
        let percentage = ev.get().await.unwrap();
        handle.emit("battery-percentage", percentage).unwrap();
      }
    });

    let handle = app.app_handle().clone();
    let display_device_handle = display_device.clone();
    tokio::spawn(async move {
      let mut stream = display_device_handle.receive_power_supply_changed().await;
      while let Some(ev) = stream.next().await {
        let psu_connected = ev.get().await.unwrap();
        handle.emit("psu-connected", psu_connected).unwrap();
      }
    });
  }

  let display = gdk::Display::default().unwrap();
  let n_monitors = display.n_monitors();

  let mut greeter_windows: Vec<tauri::WebviewWindow> = Vec::with_capacity(n_monitors as usize);
  for i in 0..n_monitors {
    let monitor = display.monitor(i).unwrap();
    let window_label = format!("greeter-{i}");
    let is_primary = i == 0;
    if !is_primary {
      continue;
    }

    let window = create_overlay_window(
      &app,
      &window_label,
      if is_primary {
        "src/login/index.html"
      } else {
        "src/login/blank.html"
      },
      &monitor,
      gtk_layer_shell::Layer::Top,
      (false, false, false, false),
      (1000, 1000),
    )
    .expect("failed to create window");

    let window_handle = window.clone();
    window.listen("ready", move |ev| {
      window_handle.unlisten(ev.id());

      if has_battery {
        window_handle.emit("has-battery", ()).unwrap();
        window_handle
          .emit("battery-percentage", percentage)
          .unwrap();
        window_handle.emit("psu-connected", psu_connected).unwrap();
      }
    });

    greeter_windows.push(window);
  }

  app.run(|_, _| {});
}

#[tauri::command]
async fn quit(app: tauri::AppHandle) {
  app.exit(0);
}

#[tauri::command]
async fn submit_password(app: tauri::AppHandle, window: tauri::WebviewWindow, value: String) {
  let uid = unsafe { libc::getuid() };
  let username = get_username(uid).unwrap();

  let conv = Box::pin(SessionConv(value));
  let mut pam = pam::session::PamSession::start("dash2", &username, conv).unwrap();

  if let Err(err) = pam.authenticate(pam_sys::PamFlag::NONE) {
    eprintln!("auth err: {err}");
    window.emit("auth-error", err.to_string()).unwrap();
    return;
  };

  if let Err(err) = pam.setcred(pam_sys::PamFlag::REFRESH_CRED) {
    eprintln!("auth setcred err: {err}");
    window.emit("auth-error", err.to_string()).unwrap();
    return;
  };

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

  gtk_window.connect_show(|window| {
    let gdk_window = window.window().unwrap().downcast::<gdkwayland::WaylandWindow>().unwrap();
    let surface = unsafe { gdk_wayland_sys::gdk_wayland_window_get_wl_surface(gdk_window.to_glib_none().0) };
    println!("showing window: {surface:?}");
  });

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
