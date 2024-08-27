use anyhow::Result;
use gdk::Monitor;
use gtk::prelude::*;
use gtk_layer_shell::LayerShell;
use webkit2gtk::WebViewExt;

pub struct LayerShellWindowBuilder {
  url: tauri::WebviewUrl,
  label: String,
  layer: Option<gtk_layer_shell::Layer>,
  edge: Option<(bool, bool, bool, bool)>,
  size: Option<(i32, i32)>,
  keyboard_mode: Option<gtk_layer_shell::KeyboardMode>,
  monitor: Option<Monitor>,
  background_color: Option<(f64, f64, f64, f64)>,
  namespace: Option<String>,
}

impl LayerShellWindowBuilder {
  pub fn new(label: &str, url: &str) -> Self {
    Self {
      label: label.to_string(),
      url: tauri::WebviewUrl::App(url.into()),
      layer: None,
      edge: None,
      size: None,
      keyboard_mode: None,
      monitor: None,
      background_color: None,
      namespace: None,
    }
  }

  pub fn layer(mut self, layer: gtk_layer_shell::Layer) -> Self {
    self.layer = Some(layer);
    self
  }

  pub fn edge(mut self, top: bool, right: bool, bottom: bool, left: bool) -> Self {
    self.edge = Some((top, right, bottom, left));
    self
  }

  pub fn size(mut self, width: i32, height: i32) -> Self {
    self.size = Some((width, height));
    self
  }

  pub fn keyboard_mode(mut self, keyboard_mode: gtk_layer_shell::KeyboardMode) -> Self {
    self.keyboard_mode = Some(keyboard_mode);
    self
  }

  pub fn monitor(mut self, monitor: &Monitor) -> Self {
    self.monitor = Some(monitor.clone());
    self
  }

  pub fn background_color(mut self, r: f64, g: f64, b: f64, a: f64) -> Self {
    self.background_color = Some((r, g, b, a));
    self
  }

  pub fn namespace(mut self, namespace: &str) -> Self {
    self.namespace = Some(namespace.to_owned());
    self
  }

  pub fn build(self, app: &tauri::AppHandle) -> Result<tauri::WebviewWindow> {
    let window = tauri::WebviewWindow::builder(app, self.label, self.url)
      .resizable(false)
      .decorations(false)
      .visible(false)
      .build()?;

    let gtk_window = window.gtk_window()?;
    gtk_window.init_layer_shell();
    gtk_window.set_app_paintable(true);

    if let Some(monitor) = self.monitor {
      gtk_window.set_monitor(&monitor);
    }

    if let Some(keyboard_mode) = self.keyboard_mode {
      gtk_window.set_keyboard_mode(keyboard_mode);
    }

    if let Some(size) = self.size {
      gtk_window.set_size_request(size.0, size.1);
    }

    if let Some(edge) = self.edge {
      let (top, right, bottom, left) = edge;
      gtk_window.set_anchor(gtk_layer_shell::Edge::Top, top);
      gtk_window.set_anchor(gtk_layer_shell::Edge::Right, right);
      gtk_window.set_anchor(gtk_layer_shell::Edge::Bottom, bottom);
      gtk_window.set_anchor(gtk_layer_shell::Edge::Left, left);
    }

    if let Some(layer) = self.layer {
      gtk_window.set_layer(layer);
    }

    if let Some(namespace) = self.namespace {
      gtk_window.set_namespace(&namespace);
    }

    let background_color = self.background_color.unwrap_or((0., 0., 0., 0.));
    window.with_webview(move |webview| {
      let webview = webview.inner();
      webview.connect_context_menu(|_, _, _, _| true);

      let (r, g, b, a) = background_color;
      webview.set_background_color(&gdk::RGBA::new(r, g, b, a));
    })?;

    window.show()?;

    Ok(window)
  }
}
