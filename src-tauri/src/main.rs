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
mod panels;
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
  Greet {
    /// Run in demo mode
    #[arg(long)]
    demo: bool,
  },

  /// Start the desktop environment
  Run,

  /// Print the configuration
  PrintConfig,
}

// TODO: ctrl-c handler
fn main() -> Result<()> {
  tracing_subscriber::fmt::init();

  let args = Args::parse();
  let config = config::load(&args.config)?;

  match args.command {
    Command::Lock => session_lock::run(config),
    Command::Greet { demo } => greeter::greet(config, demo),
    Command::Run => panels::run(config),
    Command::PrintConfig => {
      println!("{:#?}", config);
      Ok(())
    }
  }
}
