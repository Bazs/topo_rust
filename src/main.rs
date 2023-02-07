use std::{any, path::Path};

use anyhow::anyhow;
use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    // Path to the input config file.
    #[arg(short, long)]
    config_filepath: String,
}

fn try_main() -> anyhow::Result<()> {
    let args = Args::try_parse()?;
    if !Path::new(&args.config_filepath).exists() {
        return Err(anyhow!("Config file {} not found", &args.config_filepath));
    }
    Ok(())
}

fn main() {
    if let Err(e) = try_main() {
        eprintln!("Error: {:#?}", e);
        std::process::exit(1)
    }
}
