#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;
use gdk::{prelude::Cast, Monitor};
use gtk::traits::{ContainerExt, WidgetExt};
use webkit2gtk::WebViewExt;

fn main() {
  tauri::Builder::new()
    .setup(|app| {
      let display = gdk::Display::default().unwrap();
      let n_monitors = display.n_monitors();

      let mut overlay_windows = Vec::new();
      for i in 0..n_monitors {
        let monitor = display.monitor(i).unwrap();
        let monitor_geometry = monitor.geometry();

        let top = create_overlay_window(
          app,
          &format!("overlay-top-{}", i),
          "edge.html",
          &monitor,
          gtk_layer_shell::Layer::Top,
          (true, false, false, false),
          (monitor_geometry.width(), 15),
        )?;

        let bottom = create_overlay_window(
          app,
          &format!("overlay-bottom-{}", i),
          "edge.html",
          &monitor,
          gtk_layer_shell::Layer::Top,
          (false, false, true, false),
          (monitor_geometry.width(), 15),
        )?;

        overlay_windows.push((top, bottom));
      }

      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

fn create_overlay_window(
  app: &tauri::App,
  label: &str,
  url: &str,
  monitor: &Monitor,
  layer: gtk_layer_shell::Layer,
  edge: (bool, bool, bool, bool),
  size: (i32, i32),
) -> Result<tauri::Window> {
  let window = tauri::WindowBuilder::new(app, label, tauri::WindowUrl::App(url.into()))
    .resizable(false)
    .decorations(false)
    .visible(false)
    .build()?;

  let gtk_window = window.gtk_window()?;
  gtk_layer_shell::init_for_window(&gtk_window);
  gtk_layer_shell::set_monitor(&gtk_window, monitor);
  gtk_layer_shell::set_layer(&gtk_window, layer);

  let (top, right, bottom, left) = edge;

  if top {
    gtk_layer_shell::set_anchor(&gtk_window, gtk_layer_shell::Edge::Top, true);
  }
  if right {
    gtk_layer_shell::set_anchor(&gtk_window, gtk_layer_shell::Edge::Right, true);
  }
  if bottom {
    gtk_layer_shell::set_anchor(&gtk_window, gtk_layer_shell::Edge::Bottom, true);
  }
  if left {
    gtk_layer_shell::set_anchor(&gtk_window, gtk_layer_shell::Edge::Left, true);
  }

  let (width, height) = size;
  gtk_window.set_size_request(width, height);
  gtk_window.set_app_paintable(true);

  gtk_window.connect_enter_notify_event(|window, ev| {
    println!("entered");
    gtk::Inhibit(true)
  });

  gtk_window.connect_leave_notify_event(|window, ev| {
    println!("left");
    gtk::Inhibit(true)
  });

  let window_children = gtk_window.children();
  let root_box = window_children[0].downcast_ref::<gtk::Box>().unwrap();
  let root_box_children = root_box.children();
  let menu_bar = root_box_children[0].downcast_ref::<gtk::MenuBar>().unwrap();
  root_box.remove(menu_bar);

  window.with_webview(|webview| {
    let webview = webview.inner();
    webview.set_background_color(&gdk::RGBA::new(0., 0., 0., 0.));
  })?;

  window.show()?;

  Ok(window)
}
