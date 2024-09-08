use std::{thread::JoinHandle, time::Duration};

use anyhow::Result;
use gdk::{glib::translate::ToGlibPtr, prelude::*};
use smithay_client_toolkit::{
  reexports::{calloop::{channel::Channel, EventLoop}, calloop_wayland_source::WaylandSource},
  registry::{ProvidesRegistryState, RegistryState},
  registry_handlers,
};
use tracing::error;
use wayland_backend::client::Backend;
use wayland_client::{globals::registry_queue_init, protocol::wl_buffer, Connection};

use crate::config::Config;

struct State {
  config: Config,
  running: bool,
  tauri_app: tauri::AppHandle,
  registry_state: RegistryState,
}

pub fn run(
  config: Config,
  app_handle: &tauri::AppHandle,
  exit_rx: Channel<()>,
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

  event_queue.handle();

  let app_handle = app_handle.clone();
  let thread_handle = std::thread::spawn(move || {
    let mut event_loop: EventLoop<State> = match EventLoop::try_new() {
      Ok(event_loop) => event_loop,
      Err(err) => {
        error!("Failed to create event loop: {err}");
        app_handle.exit(1);
        return;
      }
    };

    let loop_handle = event_loop.handle();
    if let Err(err) = loop_handle.insert_source(exit_rx, |_, _, state| state.exit()) {
      error!("failed to insert exit source: {err}");
      app_handle.exit(1);
      return;
    }

    let mut wl_state = State {
      config,
      running: true,
      tauri_app: app_handle.clone(),
      registry_state: RegistryState::new(&globals),
    };

    if let Err(err) = WaylandSource::new(wl_conn.clone(), event_queue).insert(event_loop.handle()) {
      error!("failed to insert wayland source: {err}");
      app_handle.exit(1);
      return;
    }

    while wl_state.running {
      event_loop
        .dispatch(Duration::from_millis(16), &mut wl_state)
        .unwrap_or_else(|err| {
          error!("failed to dispatch event loop: {err}");
        });
    }

    app_handle.exit(0);
  });

  Ok(thread_handle)
}

impl State {
  fn exit(&mut self) {
    self.running = false;
  }
}

impl ProvidesRegistryState for State {
  fn registry(&mut self) -> &mut RegistryState {
    &mut self.registry_state
  }

  registry_handlers![];
}

smithay_client_toolkit::delegate_registry!(State);
wayland_client::delegate_noop!(State: ignore wl_buffer::WlBuffer);
