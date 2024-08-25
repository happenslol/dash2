use std::{
  sync::{Arc, Mutex},
  thread::JoinHandle,
  time::Duration,
};

use anyhow::Result;
use gdk::{glib::translate::ToGlibPtr, prelude::*};
use gtk::traits::WidgetExt;
use once_cell::sync::Lazy;
use smithay_client_toolkit::{
  output::{OutputHandler, OutputState},
  reexports::{
    calloop::{
      channel::{Channel, Sender},
      EventLoop, LoopHandle,
    },
    calloop_wayland_source::WaylandSource,
  },
  registry::{ProvidesRegistryState, RegistryState},
  registry_handlers,
  session_lock::{
    SessionLock, SessionLockHandler, SessionLockState, SessionLockSurface,
    SessionLockSurfaceConfigure,
  },
};
use tauri::{Emitter, Listener};
use wayland_backend::client::Backend;
use wayland_client::{
  globals::registry_queue_init,
  protocol::{
    wl_buffer,
    wl_output::{self, WlOutput},
  },
  Connection, Proxy, QueueHandle,
};
use webkit2gtk::WebViewExt;

use crate::{
  config::Config,
  util::{get_wl_surface, get_wl_window, rand_string},
};

#[derive(Clone)]
struct TauriLockSurface {
  surface: SessionLockSurface,
  window: tauri::WebviewWindow,
  output: WlOutput,
  output_name: String,
  is_active: bool,
}

struct State {
  config: Config,
  running: bool,
  loop_handle: LoopHandle<'static, Self>,
  conn: Connection,
  session_lock_state: SessionLockState,
  session_lock: Option<SessionLock>,
  tauri_app: tauri::AppHandle,
  registry_state: RegistryState,
  output_state: OutputState,
  lock_surfaces: Arc<Mutex<Vec<TauriLockSurface>>>,
  window_ready_tx: Sender<()>,
}

pub fn lock_session(
  config: Config,
  app_handle: &tauri::AppHandle,
  unlock_rx: Channel<()>,
  window_ready_tx: Sender<()>,
  window_ready_rx: Channel<()>,
) -> Result<JoinHandle<()>> {
  let display = gdk::Display::default().ok_or(anyhow::anyhow!("failed to get default display"))?;
  let wl_display = display
    .downcast_ref::<gdkwayland::WaylandDisplay>()
    .ok_or(anyhow::anyhow!("display was not a wayland display"))?;
  let wl_display =
    unsafe { gdk_wayland_sys::gdk_wayland_display_get_wl_display(wl_display.to_glib_none().0) };
  let wl_backend = unsafe { Backend::from_foreign_display(wl_display as *mut _) };
  let wl_conn = Connection::from_backend(wl_backend);
  let (globals, event_queue) = registry_queue_init(&wl_conn)?;

  let qh: QueueHandle<State> = event_queue.handle();

  let app_handle = app_handle.clone();
  let thread_handle = std::thread::spawn(move || {
    let mut event_loop: EventLoop<State> = match EventLoop::try_new() {
      Ok(event_loop) => event_loop,
      Err(err) => {
        eprintln!("Failed to create event loop: {err}");
        app_handle.exit(1);
        return;
      }
    };

    let loop_handle = event_loop.handle();

    if let Err(err) = loop_handle.insert_source(unlock_rx, |_, _, app_data| app_data.unlock()) {
      eprintln!("failed to insert unlock source: {err}");
      app_handle.exit(1);
      return;
    }

    if let Err(err) = loop_handle.insert_source(window_ready_rx, |_, _, app_data| {
      app_data.assign_primary().unwrap_or_else(|err| {
        eprintln!("failed to assign primary: {err}");
      })
    }) {
      eprintln!("failed to insert window ready source: {err}");
      app_handle.exit(1);
      return;
    }

    let mut wl_state = State {
      config,
      running: true,
      tauri_app: app_handle.clone(),
      output_state: OutputState::new(&globals, &qh),
      registry_state: RegistryState::new(&globals),
      loop_handle,
      conn: wl_conn.clone(),
      session_lock_state: SessionLockState::new(&globals, &qh),
      session_lock: None,
      lock_surfaces: Arc::new(Mutex::new(Vec::new())),
      window_ready_tx,
    };

    let session_lock = match wl_state.session_lock_state.lock(&qh) {
      Ok(session_lock) => session_lock,
      Err(err) => {
        eprintln!("Compositor does not support ext_session_lock_v1: {err}");
        app_handle.exit(1);
        return;
      }
    };

    if let Err(err) = WaylandSource::new(wl_conn.clone(), event_queue).insert(event_loop.handle()) {
      eprintln!("failed to insert wayland source: {err}");
      app_handle.exit(1);
      return;
    }

    wl_state.session_lock = Some(session_lock);
    for output in wl_state.output_state.outputs() {
      wl_state
        .create_lock_surface(&qh, &output)
        .unwrap_or_else(|err| {
          eprintln!("failed to create lock surface: {err}");
        });
    }

    while wl_state.running {
      event_loop
        .dispatch(Duration::from_millis(16), &mut wl_state)
        .unwrap_or_else(|err| {
          eprintln!("failed to dispatch event loop: {err}");
        });
    }

    app_handle.exit(0);
  });

  Ok(thread_handle)
}

static WINDOW_TITLE_RE: Lazy<regex::Regex> =
  Lazy::new(|| regex::Regex::new(r"[^a-zA-Z0-9]").expect("failed to compile regex"));

fn get_output_window_label(output: &WlOutput) -> String {
  let sanitized = WINDOW_TITLE_RE
    .replace_all(&output.id().to_string(), "")
    .to_string();

  format!("lock-{}", sanitized)
}

impl State {
  fn assign_primary(&mut self) -> Result<()> {
    let surfaces = self
      .lock_surfaces
      .lock()
      .map_err(|_| anyhow::anyhow!("failed to lock"))?;

    if surfaces.is_empty() {
      return Ok(());
    }

    let primary = self
      .config
      .primary_display
      .iter()
      .find_map(|name| {
        surfaces
          .iter()
          .find(|s| s.is_active && &s.output_name == name)
      })
      .unwrap_or(&surfaces[0]);

    surfaces
      .iter()
      .filter(|s| s.is_active && s.output != primary.output)
      .for_each(|s| {
        s.window
          .emit_to(s.window.label(), "is-primary", false)
          .unwrap_or_else(|err| {
            eprintln!("failed to emit is-primary: {err}");
          });
      });

    primary
      .window
      .emit_to(primary.window.label(), "is-primary", true)
      .unwrap_or_else(|err| {
        eprintln!("failed to emit is-primary: {err}");
      });

    primary.window.gtk_window()?.grab_focus();

    Ok(())
  }

  fn get_output_name(&mut self, output: &WlOutput) -> Result<String> {
    let Some(info) = self.output_state().info(output) else {
      return Ok(rand_string());
    };

    let result = info.name.unwrap_or_else(|| format!("{}", info.id));
    Ok(result)
  }

  fn unlock(&mut self) {
    let Some(session_lock) = self.session_lock.take() else {
      eprintln!("session lock not initialized");
      return;
    };

    session_lock.unlock();

    // Sync connection to make sure compostor receives destroy
    if let Err(err) = self.conn.roundtrip() {
      eprintln!("failed to roundtrip after unlocking session: {err}");
    };

    // Then we can exit
    self.running = false;
  }

  /// Attempts to update the name for the given output in the lock surface list.
  /// Returns true if the output was present or anything went wrong, false otherwise.
  fn refresh_output_name(&mut self, output: &wl_output::WlOutput) -> bool {
    let surfaces = self.lock_surfaces.clone();
    let Ok(mut surfaces) = surfaces.lock() else {
      eprintln!("failed to lock surfaces for new output");
      return true;
    };

    if let Some(found) = surfaces.iter_mut().find(|s| s.output == *output) {
      let output_name = match self.get_output_name(output) {
        Ok(output_name) => output_name,
        Err(err) => {
          eprintln!("failed to get output name: {err}");
          return true;
        }
      };

      found.output_name = output_name;
      return true;
    }

    false
  }

  fn create_lock_surface(&mut self, qh: &QueueHandle<Self>, output: &WlOutput) -> Result<()> {
    let output_name = self.get_output_name(output)?;

    let Some(session_lock) = self.session_lock.as_ref() else {
      anyhow::bail!("session lock not initialized");
    };

    let window_label = get_output_window_label(output);
    let window = tauri::WebviewWindow::builder(
      &self.tauri_app,
      window_label,
      tauri::WebviewUrl::App("src/login/index.html".into()),
    )
    .visible(false)
    .build()?;

    window.with_webview(|webview| {
      let webview = webview.inner();
      webview.set_background_color(&gdk::RGBA::new(0., 0., 0., 0.));
    })?;

    let qh = qh.clone();
    let session_lock = session_lock.clone();

    let gtk_window = window.gtk_window()?;

    let conn = self.conn.clone();
    gtk_window.connect_map(move |window| {
      let Ok(wl_window) = get_wl_window(window) else {
        return;
      };

      let Ok(surface) = get_wl_surface(&conn, &wl_window) else {
        return;
      };

      surface.attach(None, 0, 0);
    });

    let ev_window = window.clone();
    let window_ready_tx = self.window_ready_tx.clone();
    window.listen("ready", move |ev| {
      // ev_window.open_devtools();
      ev_window.unlisten(ev.id());
      window_ready_tx.send(()).unwrap_or_else(|err| {
        eprintln!("failed to send window ready: {err}");
      });
    });

    let conn = self.conn.clone();
    let surfaces = self.lock_surfaces.clone();
    let output = output.clone();
    gtk_window.connect_realize(move |gtk_window| {
      let Ok(wl_window) = get_wl_window(gtk_window) else {
        return;
      };

      unsafe {
        gdk_wayland_sys::gdk_wayland_window_set_use_custom_surface(wl_window.to_glib_none().0);
      }

      let Ok(surface) = get_wl_surface(&conn, &wl_window) else {
        return;
      };

      let lock_surface = session_lock.create_lock_surface(surface, &output, &qh);

      {
        let Ok(mut surfaces) = surfaces.lock() else {
          return;
        };

        surfaces.push(TauriLockSurface {
          surface: lock_surface,
          window: window.clone(),
          output: output.clone(),
          output_name: output_name.clone(),
          is_active: true,
        });
      }

      gtk_window.hide();
    });

    gtk_window.realize();

    Ok(())
  }
}

impl ProvidesRegistryState for State {
  fn registry(&mut self) -> &mut RegistryState {
    &mut self.registry_state
  }
  registry_handlers![OutputState,];
}

impl OutputHandler for State {
  fn output_state(&mut self) -> &mut OutputState {
    &mut self.output_state
  }

  fn new_output(
    &mut self,
    _conn: &Connection,
    qh: &QueueHandle<Self>,
    output: wl_output::WlOutput,
  ) {
    if self.refresh_output_name(&output) {
      self.assign_primary().unwrap_or_else(|err| {
        eprintln!("failed to assign primary: {err}");
      });
      return;
    }

    self.create_lock_surface(qh, &output).unwrap_or_else(|err| {
      eprintln!("failed to create lock surface: {err}");
    })
  }

  fn update_output(
    &mut self,
    _conn: &Connection,
    _qh: &QueueHandle<Self>,
    output: wl_output::WlOutput,
  ) {
    self.refresh_output_name(&output);

    self.assign_primary().unwrap_or_else(|err| {
      eprintln!("failed to assign primary: {err}");
    });
  }

  fn output_destroyed(
    &mut self,
    _conn: &Connection,
    _qh: &QueueHandle<Self>,
    output: wl_output::WlOutput,
  ) {
    {
      let Ok(mut surfaces) = self.lock_surfaces.lock() else {
        eprintln!("failed to lock surfaces for destroyed output");
        return;
      };

      let Some(found) = surfaces.iter_mut().find(|s| s.output == output) else {
        eprintln!("no surface found for destroyed output");
        return;
      };

      found.is_active = false;

      let Ok(_) = found.window.close() else {
        eprintln!("failed to close window for destroyed output");
        return;
      };
    }

    self.assign_primary().unwrap_or_else(|err| {
      eprintln!("failed to assign primary: {err}");
    });
  }
}

impl SessionLockHandler for State {
  fn locked(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _session_lock: SessionLock) {
    self.assign_primary().unwrap_or_else(|err| {
      eprintln!("failed to assign primary: {err}");
    })
  }

  fn finished(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _session_lock: SessionLock) {
    self.running = false;
  }

  fn configure(
    &mut self,
    _conn: &Connection,
    _qh: &QueueHandle<Self>,
    session_lock_surface: SessionLockSurface,
    configure: SessionLockSurfaceConfigure,
    _serial: u32,
  ) {
    {
      let Ok(surfaces) = self.lock_surfaces.lock() else {
        eprintln!("failed to lock surfaces for configure");
        return;
      };

      let found = surfaces
        .iter()
        .find(|s| s.surface.wl_surface() == session_lock_surface.wl_surface());

      if let Some(found) = found {
        let (width, height) = configure.new_size;
        if let Ok(gtk_window) = found.window.gtk_window() {
          gtk_window.set_size_request(width as i32, height as i32);
        }

        found.window.show().unwrap_or_else(|err| {
          eprintln!("failed to show window: {err}");
        });
      }
    }

    self.assign_primary().unwrap_or_else(|err| {
      eprintln!("failed to assign primary: {err}");
    })
  }
}

smithay_client_toolkit::delegate_output!(State);
smithay_client_toolkit::delegate_session_lock!(State);
smithay_client_toolkit::delegate_registry!(State);
wayland_client::delegate_noop!(State: ignore wl_buffer::WlBuffer);
