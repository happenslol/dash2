#![allow(dead_code)]

use std::process::ExitCode;

pub(crate) mod battery;
pub(crate) mod hyprland;
pub(crate) mod layer_shell;
pub(crate) mod pam;
pub(crate) mod scrambler;
mod session_lock;
pub(crate) mod util;

fn main() -> ExitCode {
  if let Err(err) = session_lock::run() {
    eprintln!("Failed to run session lock {}", err);
    return ExitCode::FAILURE;
  }

  ExitCode::SUCCESS
}
