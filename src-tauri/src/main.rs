#![allow(dead_code)]

use std::process::ExitCode;

pub mod battery;
pub mod hyprland;
pub mod layer_shell;
pub mod pam;
pub mod power;
pub mod scrambler;
mod session_lock;
pub mod util;

fn main() -> ExitCode {
  if let Err(err) = session_lock::run() {
    eprintln!("Failed to run session lock {}", err);
    return ExitCode::FAILURE;
  }

  ExitCode::SUCCESS
}
