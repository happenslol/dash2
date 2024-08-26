#![allow(dead_code)]

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

pub mod battery;
pub mod config;
pub mod hyprland;
pub mod layer_shell;
pub mod pam;
pub mod power;
pub mod scrambler;
pub mod util;

mod greeter;
mod session_lock;

#[derive(Debug, Parser)]
struct Args {
  /// Path to the configuration file
  config: Option<PathBuf>,

  #[command(subcommand)]
  command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
  /// Lock the session
  Lock,

  /// Start the greeter
  Greet,

  /// Print the configuration
  PrintConfig,
}

// TODO: ctrl-c handler
// TODO: tracing
fn main() -> Result<()> {
  let args = Args::parse();
  let config = config::load(&args.config)?;

  match args.command {
    Command::Lock => session_lock::run(config),
    Command::Greet => greeter::greet(config),
    Command::PrintConfig => {
      println!("{:#?}", config);
      Ok(())
    }
  }
}
