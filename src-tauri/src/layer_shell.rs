use anyhow::Result;
use gdk::Monitor;
use gtk::prelude::*;
use gtk_layer_shell::LayerShell;
use webkit2gtk::WebViewExt;

pub fn create_overlay_window(
  app: &tauri::AppHandle,
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
