use serde::Deserialize;
use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  net::UnixStream,
};

#[derive(Debug, Deserialize)]
struct HLMonitor {
  id: i32,
  name: String,
  model: String,
  width: i32,
  height: i32,
  disabled: bool,
}

async fn get_hyprland_monitors() -> Vec<HLMonitor> {
  let his = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").unwrap();
  let rt_dir = std::env::var("XDG_RUNTIME_DIR").unwrap();
  let mut socket = UnixStream::connect(format!("{rt_dir}/hypr/{his}/.socket.sock"))
    .await
    .unwrap();

  socket.write_all(b"j/monitors").await.unwrap();
  let mut res = String::new();
  socket.read_to_string(&mut res).await.unwrap();

  serde_json::from_str::<Vec<HLMonitor>>(&res).unwrap()
}
