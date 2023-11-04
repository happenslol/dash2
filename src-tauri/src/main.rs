#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use gdk::prelude::Cast;
use gtk::traits::{ContainerExt, WidgetExt};
use webkit2gtk::WebViewExt;

fn main() {
  tauri::Builder::new()
    .setup(|app| {
      let default_window =
        tauri::WindowBuilder::new(app, "default", tauri::WindowUrl::App("index.html".into()))
          .visible(false)
          .decorations(false)
          .build()?;

      let gtk_window = default_window.gtk_window()?;
      gtk_layer_shell::init_for_window(&gtk_window);
      gtk_layer_shell::set_layer(&gtk_window, gtk_layer_shell::Layer::Overlay);
      gtk_layer_shell::set_anchor(&gtk_window, gtk_layer_shell::Edge::Bottom, true);

      gtk_window.set_app_paintable(true);
      gtk_window.set_size_request(500, 500);

      let window_children = gtk_window.children();
      let root_box = window_children[0].downcast_ref::<gtk::Box>().unwrap();
      let root_box_children = root_box.children();
      let menu_bar = root_box_children[0].downcast_ref::<gtk::MenuBar>().unwrap();
      root_box.remove(menu_bar);

      default_window.with_webview(|webview| {
        let webview = webview.inner();
        webview.set_background_color(&gdk::RGBA::new(0., 0., 0., 0.));
      })?;

      default_window.show()?;
      default_window.open_devtools();

      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
