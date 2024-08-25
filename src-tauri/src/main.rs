#![allow(dead_code)]

use std::process::ExitCode;

use clap::{Parser, Subcommand};

pub mod battery;
pub mod hyprland;
pub mod layer_shell;
pub mod pam;
pub mod power;
pub mod scrambler;
mod session_lock;
pub mod util;

#[derive(Debug, Parser)]
struct Args {
  #[command(subcommand)]
  command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
  /// Lock the session
  Lock,
}

fn main() -> ExitCode {
  let args = Args::parse();
  match args.command {
    Command::Lock => {
      if let Err(err) = session_lock::run() {
        eprintln!("Failed to run session lock {}", err);
        return ExitCode::FAILURE;
      }
    }
  }

  ExitCode::SUCCESS
}
