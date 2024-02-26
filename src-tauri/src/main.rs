#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;
use gdk::{prelude::Cast, Monitor};
use gtk::{
  prelude::GtkWindowExt,
  traits::{ContainerExt, WidgetExt},
};
use webkit2gtk::WebViewExt;

fn main() {
  let app = tauri::Builder::default()
    .build(tauri::generate_context!())
    .expect("error while building tauri application");

  let display = gdk::Display::default().unwrap();

  let init_window = tauri::WindowBuilder::new(&app, "init", tauri::WindowUrl::App("".into()))
    .visible(false)
    .build()
    .expect("failed to create init window");

  let gtk_app = init_window
    .gtk_window()
    .expect("failed to get window for init app")
    .application()
    .expect("failed to get application from init window");

  init_window.close().unwrap();

  let main_monitor = display.monitor(0).unwrap();
  let top_window = gtk::ApplicationWindow::new(&gtk_app);

  gtk_layer_shell::init_for_window(&top_window);
  gtk_layer_shell::set_monitor(&top_window, &main_monitor);
  gtk_layer_shell::set_layer(&top_window, gtk_layer_shell::Layer::Top);

  gtk_layer_shell::set_anchor(&top_window, gtk_layer_shell::Edge::Top, true);
  gtk_layer_shell::set_anchor(&top_window, gtk_layer_shell::Edge::Right, true);
  gtk_layer_shell::set_anchor(&top_window, gtk_layer_shell::Edge::Left, true);

  top_window.set_resizable(false);
  top_window.set_decorated(false);
  let width = main_monitor.geometry().width();
  top_window.set_size_request(width, 15);
  top_window.set_app_paintable(true);

  top_window.show_all();

  create_overlay_window(
    &app,
    "test",
    "test.html",
    &main_monitor,
    gtk_layer_shell::Layer::Top,
    (false, false, false, false),
    (400, 400),
  )
  .expect("failed to create test window");

  top_window.connect_enter_notify_event(|_window, _ev| {
    println!("entered top window");
    gtk::Inhibit(true)
  });

  app.run(|_, _| {});
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

  gtk_window.connect_enter_notify_event(|_window, _ev| {
    println!("entered");
    gtk::Inhibit(true)
  });

  gtk_window.connect_leave_notify_event(|_window, _ev| {
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
