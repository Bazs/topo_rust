extern crate log;
pub mod geofile;
pub mod osm;
use crate::osm::download::{sync_osm_data_to_file, WgsBoundingBox};
use anyhow::anyhow;
use clap::Parser;
use serde::Deserialize;
use std::{fs::read_to_string, path::Path};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the input config file.
    #[arg(short, long)]
    config_filepath: String,
}

#[derive(Deserialize, Debug)]
struct Config<'a> {
    osm_bounding_box: WgsBoundingBox,
    data_dir: &'a str,
}

fn try_main() -> anyhow::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }

    let args = Args::try_parse()?;
    if !Path::new(&args.config_filepath).exists() {
        return Err(anyhow!("Config file {} not found", &args.config_filepath));
    }
    let config_contents = read_to_string(args.config_filepath)?;
    let config: Config = serde_yaml::from_str(&config_contents)?;
    log::info!(
        "Syncing OSM data for bounding box {:?}",
        config.osm_bounding_box
    );
    let osm_filepath =
        sync_osm_data_to_file(&config.osm_bounding_box, Path::new(&config.data_dir))?;
    log::info!("Reading OSM ways");
    let ways = osm::conversion::read_osm_ways_from_file(&osm_filepath)?;
    log::info!("Read {} OSM ways", ways.len());
    let geojson_dump_filepath = osm_filepath.with_extension("geojson");
    log::info!("Writing ways to GeoJSON to {:?}", &geojson_dump_filepath);
    geofile::geojson::write_lines_to_geojson(&ways, &geojson_dump_filepath)?;
    Ok(())
}

fn main() {
    env_logger::init();
    if let Err(e) = try_main() {
        eprintln!("Error: {:?}", e);
        std::process::exit(1)
    }
}
