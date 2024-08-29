use anyhow::Result;
use futures::{channel::mpsc::channel, SinkExt};
use serde::de::DeserializeOwned;
use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  net::UnixStream,
};
use tracing::error;

use self::{
  data::Monitor,
  dispatch::DataCommand,
  events::{parse_event, Event},
  types::{CommandContent, CommandFlag},
};

pub mod data;
pub mod dispatch;
pub mod events;
pub mod types;

fn get_socket_path(socket_name: &str) -> Result<String> {
  let his = std::env::var("HYPRLAND_INSTANCE_SIGNATURE")?;
  let rt_dir = std::env::var("XDG_RUNTIME_DIR")?;
  Ok(format!("{rt_dir}/hypr/{his}/{socket_name}"))
}

pub struct HyprlandClient {
  path: String,
}

impl HyprlandClient {
  pub async fn new() -> Result<Self> {
    let path = get_socket_path(".socket.sock")?;
    if !std::path::Path::new(&path).exists() {
      return Err(anyhow::anyhow!("hyprland client socket not found"));
    }

    Ok(Self { path })
  }

  pub async fn get_monitors(&self) -> Result<Vec<Monitor>> {
    self.call_data_command(DataCommand::Monitors).await
  }

  async fn call_data_command<T: DeserializeOwned>(&self, cmd: DataCommand) -> Result<T> {
    let mut socket = UnixStream::connect(&self.path).await?;
    let cmd = CommandContent {
      flag: CommandFlag::JSON,
      data: cmd.to_string(),
    };

    socket.write_all(&cmd.as_bytes()).await?;

    let mut response = Vec::new();

    const BUF_SIZE: usize = 8192;
    let mut buf = [0; BUF_SIZE];
    loop {
      let num_read = socket.read(&mut buf).await?;
      let buf = &buf[..num_read];
      response.append(&mut buf.to_vec());
      if num_read == 0 || num_read != BUF_SIZE {
        break;
      }
    }

    let response = String::from_utf8(response)?;
    Ok(serde_json::from_str::<T>(&response)?)
  }
}

pub struct HyprlandListener {
  path: String,
}

impl HyprlandListener {
  pub async fn new() -> Result<Self> {
    let path = get_socket_path(".socket2.sock")?;
    if !std::path::Path::new(&path).exists() {
      return Err(anyhow::anyhow!("hyprland event socket not found"));
    }

    Ok(Self { path })
  }

  pub async fn listen(&self) -> Result<impl futures::Stream<Item = Event>> {
    let mut socket = UnixStream::connect(&self.path).await?;
    let (mut tx, rx) = channel::<Event>(5);

    tokio::spawn(async move {
      loop {
        let mut buf = [0; 4096];
        let num_read = match socket.read(&mut buf).await {
          Ok(n) => n,
          Err(e) => {
            error!("Error reading from socket: {e}");
            break;
          }
        };

        if num_read == 0 {
          break;
        }
        let buf = &buf[..num_read];
        let string = String::from_utf8(buf.to_vec()).unwrap();
        let parsed: Vec<Event> = parse_event(string).unwrap();

        for event in parsed {
          tx.send(event).await.unwrap();
        }
      }
    });

    Ok(rx)
  }
}
