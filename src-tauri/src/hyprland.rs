use anyhow::Result;
use serde::{de::DeserializeOwned, Deserialize};
use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  net::UnixStream,
};

pub struct HyprlandConn {
  path: String,
}

impl HyprlandConn {
  pub async fn new() -> Result<Self> {
    let his = std::env::var("HYPRLAND_INSTANCE_SIGNATURE")?;
    let rt_dir = std::env::var("XDG_RUNTIME_DIR")?;
    let path = format!("{rt_dir}/hypr/{his}/.socket.sock");

    Ok(Self { path })
  }

  pub async fn get_monitors(&self) -> Result<Vec<HLMonitor>> {
    self.query(b"j/monitors").await
  }

  async fn query<T: DeserializeOwned>(&self, query: &[u8]) -> Result<T> {
    let mut socket = UnixStream::connect(&self.path).await?;
    socket.write_all(query).await.unwrap();
    let mut res = String::new();
    socket.read_to_string(&mut res).await.unwrap();
    serde_json::from_str::<T>(&res).map_err(Into::into)
  }
}

#[derive(Debug, Deserialize)]
pub struct HLMonitor {
  pub id: i32,
  pub name: String,
  pub model: String,
  pub width: i32,
  pub height: i32,
  pub disabled: bool,
}
