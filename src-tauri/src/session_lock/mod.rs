use anyhow::Result;
use smithay_client_toolkit::reexports::calloop::channel::{channel, Sender};
use tauri::{Emitter, Manager};

use crate::{
  battery::{BatteryState, BatterySubscription},
  pam,
  util::get_current_username,
};

mod wayland;

pub fn run() -> Result<()> {
  let rt = tokio::runtime::Runtime::new()?;
  rt.block_on(async {
    tauri::async_runtime::set(tokio::runtime::Handle::current());

    let app = tauri::Builder::default()
      .invoke_handler(tauri::generate_handler![
        poweroff,
        submit_password,
        suspend,
        get_battery_state
      ])
      .build(tauri::generate_context!())
      .expect("error while building tauri application");

    let zbus_conn = zbus::Connection::system().await?;
    let battery = BatterySubscription::new(app.handle(), &zbus_conn).await?;

    let (unlock_tx, unlock_rx) = channel();
    app.manage(TauriState { unlock_tx, battery });

    let lock_handle = wayland::lock_session(app.handle(), unlock_rx)?;

    app.run(|_, _| {});
    lock_handle
      .join()
      .expect("error while joining lock session");

    Ok(())
  })
}

struct TauriState<'a> {
  unlock_tx: Sender<()>,
  battery: BatterySubscription<'a>,
}

#[tauri::command]
async fn poweroff() {}

#[tauri::command]
async fn suspend() {}

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
      .unwrap_or_else(|err| eprintln!("failed to emit: {err}"));
    return;
  };

  let authenticated = tokio::task::spawn_blocking(move || {
    let conv = Box::pin(pam::PasswordConv::new(value));
    let Ok(mut pam) = pam::session::PamSession::start("dash2", &username, conv) else {
      window
        .emit("password-error", "failed to start pam session")
        .unwrap_or_else(|err| eprintln!("failed to emit: {err}"));
      return false;
    };

    if let Err(err) = pam.authenticate(pam_sys::PamFlag::NONE) {
      window
        .emit("password-error", err.to_string())
        .unwrap_or_else(|err| eprintln!("failed to emit: {err}"));
      return false;
    };

    if let Err(err) = pam.setcred(pam_sys::PamFlag::REFRESH_CRED) {
      window
        .emit("password-error", err.to_string())
        .unwrap_or_else(|err| eprintln!("failed to emit: {err}"));
      return false;
    };

    true
  })
  .await
  .unwrap_or_else(|err| {
    eprintln!("failed to authenticate: {err}");
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
      eprintln!("failed to send unlock signal: {err}");
    })
}
