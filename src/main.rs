pub mod osm;

use std::{fs::read_to_string, path::Path};

use anyhow::anyhow;
use clap::Parser;
use yaml_rust::YamlLoader;

use crate::osm::download::download_osm_data_by_bbox;

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
    let config_contents = read_to_string(args.config_filepath)?;
    let config = &YamlLoader::load_from_str(&config_contents)?[0];
    dbg!(&config);
    let bounding_box = &config["osm_bounding_box"];
    let osm_data = download_osm_data_by_bbox(
        bounding_box["left_lon"].as_f64().unwrap(),
        bounding_box["bottom_lat"].as_f64().unwrap(),
        bounding_box["right_lon"].as_f64().unwrap(),
        bounding_box["top_lat"].as_f64().unwrap(),
    )?;
    dbg!(&osm_data);
    Ok(())
}

fn main() {
    if let Err(e) = try_main() {
        eprintln!("Error: {:#?}", e);
        std::process::exit(1)
    }
}
