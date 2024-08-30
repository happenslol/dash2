use std::time::Duration;

use anyhow::Result;
use smithay_client_toolkit::reexports::calloop::channel::{channel, Sender};
use tauri::{Emitter, Manager};
use tracing::error;

use crate::{
  battery::{BatteryState, BatterySubscription},
  config::Config,
  pam,
  power::Power,
  util::get_current_username,
};

mod wayland;

struct TauriState<'a> {
  config: Config,
  unlock_tx: Sender<()>,
  window_ready_tx: Sender<()>,
  battery: BatterySubscription<'a>,
  power: Power,
}

pub fn run(config: Config, should_suspend: bool) -> Result<()> {
  let rt = tokio::runtime::Runtime::new()?;
  rt.block_on(async {
    tauri::async_runtime::set(tokio::runtime::Handle::current());

    let app = tauri::Builder::default()
      .invoke_handler(tauri::generate_handler![
        poweroff,
        submit_password,
        suspend,
        get_battery_state,
        window_ready,
      ])
      .build(tauri::generate_context!())?;

    let zbus_conn = zbus::Connection::system().await?;
    let battery = BatterySubscription::new(app.handle(), &zbus_conn).await?;
    let power = Power::new(zbus_conn);

    let (unlock_tx, unlock_rx) = channel();
    let (window_ready_tx, window_ready_rx) = channel();
    app.manage(TauriState {
      config: config.clone(),
      window_ready_tx: window_ready_tx.clone(),
      unlock_tx,
      battery,
      power,
    });

    let lock_handle = wayland::lock_session(
      config,
      app.handle(),
      unlock_rx,
      window_ready_tx,
      window_ready_rx,
    )?;

    if should_suspend {
      let handle = app.handle().clone();
      tokio::spawn(async move {
        let state = handle.state::<TauriState>();
        tokio::time::sleep(Duration::from_millis(500)).await;
        state.power.suspend().await.unwrap_or_else(|err| {
          error!("failed to suspend: {err}");
        });
      });
    }

    app.run(|_, _| {});
    lock_handle
      .join()
      .expect("error while joining lock session");

    Ok(())
  })
}

#[tauri::command]
async fn window_ready(app: tauri::AppHandle) {
  let state = app.state::<TauriState>();
  state.window_ready_tx.send(()).unwrap_or_else(|err| {
    error!("failed to send window ready signal: {err}");
  });
}

#[tauri::command]
async fn poweroff(app: tauri::AppHandle) {
  let state = app.state::<TauriState>();
  state.power.poweroff().await.unwrap_or_else(|err| {
    error!("failed to poweroff: {err}");
  });
}

#[tauri::command]
async fn reboot(app: tauri::AppHandle) {
  let state = app.state::<TauriState>();
  state.power.reboot().await.unwrap_or_else(|err| {
    error!("failed to poweroff: {err}");
  });
}

#[tauri::command]
async fn suspend(app: tauri::AppHandle) {
  let state = app.state::<TauriState>();
  state.power.suspend().await.unwrap_or_else(|err| {
    error!("failed to suspend: {err}");
  });
}

#[tauri::command]
async fn get_battery_state(app: tauri::AppHandle) -> Option<BatteryState> {
  let state = app.state::<TauriState>();
  state.battery.get_state().await.unwrap_or(None)
}

#[tauri::command]
async fn submit_password(app: tauri::AppHandle, window: tauri::WebviewWindow, value: String) {
  let Some(username) = get_current_username() else {
    window
      .emit("password-error", "username not available")
      .unwrap_or_else(|err| error!("failed to emit: {err}"));
    return;
  };

  let authenticated = tokio::task::spawn_blocking(move || {
    let conv = Box::pin(pam::PasswordConv::new(value));
    let Ok(mut pam) = pam::session::PamSession::start("dash2", &username, conv) else {
      window
        .emit("password-error", "failed to start pam session")
        .unwrap_or_else(|err| error!("failed to emit: {err}"));
      return false;
    };

    if let Err(err) = pam.authenticate(pam_sys::PamFlag::NONE) {
      window
        .emit("password-error", err.to_string())
        .unwrap_or_else(|err| error!("failed to emit: {err}"));
      return false;
    };

    if let Err(err) = pam.setcred(pam_sys::PamFlag::REFRESH_CRED) {
      window
        .emit("password-error", err.to_string())
        .unwrap_or_else(|err| error!("failed to emit: {err}"));
      return false;
    };

    true
  })
  .await
  .unwrap_or_else(|err| {
    error!("failed to authenticate: {err}");
    false
  });

  if !authenticated {
    return;
  }

  app
    .state::<TauriState>()
    .unlock_tx
    .send(())
    .unwrap_or_else(|err| {
      error!("failed to send unlock signal: {err}");
    })
}
