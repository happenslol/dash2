use anyhow::Result;
use futures::StreamExt;
use serde::Serialize;
use tauri::Emitter;
use tracing::error;

#[derive(Copy, Clone, Serialize)]
pub struct BatteryState {
  pub percentage: f64,
  pub psu_connected: bool,
}

pub struct BatterySubscription<'a> {
  zbus: zbus::Connection,
  upower: upower_dbus::UPowerProxy<'a>,
}

impl<'a> BatterySubscription<'a> {
  pub async fn new(app_handle: &tauri::AppHandle, zbus_conn: &zbus::Connection) -> Result<Self> {
    let upower = upower_dbus::UPowerProxy::new(zbus_conn).await?;

    let display_device = upower.get_display_device().await?;
    let ttype = display_device.type_().await?;

    if ttype == upower_dbus::BatteryType::Battery {
      let handle = app_handle.clone();
      let display_device_handle = display_device.clone();
      tokio::spawn(async move {
        let mut stream = display_device_handle.receive_percentage_changed().await;
        while let Some(ev) = stream.next().await {
          match ev.get().await {
            Ok(percentage) => handle
              .emit("battery-percentage", percentage)
              .unwrap_or_else(|e| {
                error!("Failed to emit battery percentage: {}", e);
              }),
            Err(e) => error!("Failed to get battery percentage: {}", e),
          }
        }
      });

      let handle = app_handle.clone();
      let display_device_handle = display_device.clone();
      tokio::spawn(async move {
        let mut stream = display_device_handle.receive_power_supply_changed().await;
        while let Some(ev) = stream.next().await {
          match ev.get().await {
            Ok(psu_connected) => handle
              .emit("psu-connected", psu_connected)
              .unwrap_or_else(|e| {
                error!("Failed to emit psu connected: {}", e);
              }),
            Err(e) => error!("Failed to get psu connected: {}", e),
          }
        }
      });
    }

    Ok(Self {
      zbus: zbus_conn.clone(),
      upower,
    })
  }

  pub async fn get_state(&self) -> Result<Option<BatteryState>> {
    let display_device = self.upower.get_display_device().await?;
    let ttype = display_device.type_().await?;
    if ttype != upower_dbus::BatteryType::Battery {
      return Ok(None);
    }

    let percentage = display_device.percentage().await?;
    let psu_connected = display_device.power_supply().await?;

    Ok(Some(BatteryState {
      percentage,
      psu_connected,
    }))
  }
}
