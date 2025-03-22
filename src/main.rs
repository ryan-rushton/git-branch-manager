use clap::Parser;
use color_eyre::eyre::Result;

use crate::{
  app::App,
  cli::Cli,
  utils::{initialize_logging, initialize_panic_handler},
};

pub mod action;
pub mod app;
pub mod cli;
pub mod components;
pub mod config;
pub mod error;
pub mod git;
pub mod mode;
pub mod tui;
pub mod utils;

async fn tokio_main() -> Result<()> {
  initialize_logging()?;
  initialize_panic_handler()?;

  Cli::parse();

  match App::new() {
    Ok(mut app) => app.run().await?,
    Err(e) => {
      if e.to_string().contains("Not a git repository") {
        eprintln!("Error: The current directory is not a git repository.");
        std::process::exit(1);
      }
      return Err(e);
    },
  }

  Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
  if let Err(e) = tokio_main().await {
    eprintln!("{} error: Something went wrong", env!("CARGO_PKG_NAME"));
    Err(e)
  } else {
    Ok(())
  }
}
