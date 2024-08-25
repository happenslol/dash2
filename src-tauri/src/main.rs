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
mod session_lock;
pub mod util;

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

  /// Print the configuration
  PrintConfig,
}

fn main() -> Result<()> {
  let args = Args::parse();
  let config = config::load(&args.config)?;

  match args.command {
    Command::Lock => session_lock::run(config),
    Command::PrintConfig => {
      println!("{:#?}", config);
      Ok(())
    }
  }
}
