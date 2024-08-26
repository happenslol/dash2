use anyhow::{Context, Result};
use greetd_ipc::codec::TokioCodec;
use tokio::net::UnixStream;

use crate::config::Config;

pub struct GreetdClient {
  config: Config,
  socket: UnixStream,
}

impl GreetdClient {
  pub async fn new(config: Config) -> Result<Self> {
    let sock_path =
      std::env::var("GREETD_SOCK").context("Missing env var GREETD_SOCK. Is greetd running?")?;
    let socket = UnixStream::connect(sock_path).await?;

    Ok(Self { config, socket })
  }

  pub async fn authenticate(&mut self, password: String) -> Result<()> {
    let msg = greetd_ipc::Request::CreateSession {
      username: self.config.user.clone(),
    };

    msg.write_to(&mut self.socket).await?;
    match greetd_ipc::Response::read_from(&mut self.socket).await? {
      greetd_ipc::Response::Success | greetd_ipc::Response::AuthMessage { .. } => Ok(()),
      greetd_ipc::Response::Error {
        error_type,
        description,
      } => Err(anyhow::anyhow!("auth error: {error_type:?}: {description}")),
    }?;

    let msg = greetd_ipc::Request::PostAuthMessageResponse {
      response: Some(password),
    };

    msg.write_to(&mut self.socket).await?;
    match greetd_ipc::Response::read_from(&mut self.socket).await? {
      greetd_ipc::Response::Success | greetd_ipc::Response::AuthMessage { .. } => Ok(()),
      greetd_ipc::Response::Error {
        error_type,
        description,
      } => Err(anyhow::anyhow!("auth error: {error_type:?}: {description}")),
    }?;

    let msg = greetd_ipc::Request::StartSession {
      cmd: vec![String::from("echo"), String::from("hello world")],
      env: vec![],
    };

    msg.write_to(&mut self.socket).await?;
    match greetd_ipc::Response::read_from(&mut self.socket).await? {
      greetd_ipc::Response::Success => Ok(()),
      greetd_ipc::Response::AuthMessage { .. } => {
        Err(anyhow::anyhow!("got auth message in start_session"))
      }
      greetd_ipc::Response::Error {
        error_type,
        description,
      } => Err(anyhow::anyhow!("auth error: {error_type:?}: {description}")),
    }?;

    Ok(())
  }
}
