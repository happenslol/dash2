use anyhow::Result;

pub struct Power {
  zbus: zbus::Connection,
}

impl Power {
  pub fn new(zbus_conn: zbus::Connection) -> Self {
    Self { zbus: zbus_conn }
  }

  pub async fn poweroff(&self) -> Result<()> {
    self
      .send(
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
        "PowerOff",
        &(true),
      )
      .await
  }

  pub async fn reboot(&self) -> Result<()> {
    self
      .send(
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
        "Reboot",
        &(true),
      )
      .await
  }

  pub async fn suspend(&self) -> Result<()> {
    self
      .send(
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
        "Suspend",
        &(true),
      )
      .await
  }

  async fn has_owner(&self, name: &str) -> Result<bool> {
    self
      .zbus
      .call_method(
        Some("org.freedesktop.DBus"),
        "/",
        Some("org.freedesktop.DBus"),
        "NameHasOwner",
        &(name),
      )
      .await
      .and_then(|r| r.body::<bool>())
      .map_err(Into::into)
  }

  async fn send<T: zbus::export::serde::Serialize + zbus::zvariant::DynamicType>(
    &self,
    dest: &str,
    path: &str,
    interface: &str,
    method: &str,
    body: &T,
  ) -> Result<()> {
    if !self.has_owner(dest).await? {
      anyhow::bail!("no dbus owner for {dest}");
    }

    let reply = self
      .zbus
      .call_method(Some(dest), path, Some(interface), method, body)
      .await;

    if let Err(zbus::Error::MethodError(ref name, _, _)) = reply {
      // Code 19 is G_IO_ERROR_CANCELLED
      if name.contains("org.gtk.GDBus.UnmappedGError.Quark") && name.contains(".Code19") {
        return Ok(());
      }
    }

    reply.map(|_| ()).map_err(Into::into)
  }
}
