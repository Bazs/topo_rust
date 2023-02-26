extern crate log;
pub mod geofile;
pub mod osm;
pub mod topo;
use crate::geofile::geojson::read_lines_from_geojson;
use crate::osm::download::{sync_osm_data_to_file, WgsBoundingBox};
use crate::topo::topo::{calculate_topo, TopoParams};
use anyhow::anyhow;
use clap::Parser;
use serde::Deserialize;
use std::path::PathBuf;
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
enum GroundTruthConfig {
    Geojson { filepath: PathBuf },
    Osm { bounding_box: WgsBoundingBox },
}

#[derive(Deserialize, Debug)]
struct Config {
    proposal_geojson_path: PathBuf,
    ground_truth: GroundTruthConfig,
    data_dir: PathBuf,
}

fn get_ground_truth_from_osm(
    bounding_box: &WgsBoundingBox,
    data_dir: &PathBuf,
) -> anyhow::Result<Vec<geo::LineString>> {
    log::info!("Syncing OSM data for bounding box {:?}", bounding_box);
    let osm_filepath = sync_osm_data_to_file(&bounding_box, &data_dir)?;
    log::info!("Reading OSM ways");
    osm::conversion::read_osm_ways_from_file(&osm_filepath)
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

    let ground_truth_ways = match config.ground_truth {
        GroundTruthConfig::Osm { bounding_box } => {
            get_ground_truth_from_osm(&bounding_box, &config.data_dir)?
        }
        GroundTruthConfig::Geojson { filepath } => read_lines_from_geojson(&filepath)?,
    };
    log::info!("Read {} ground truth edges", ground_truth_ways.len());
    let proposal_ways = read_lines_from_geojson(&config.proposal_geojson_path)?;
    log::info!("Read {} proposal edges", ground_truth_ways.len());

    let geojson_dump_filepath = config.data_dir.join("ground_truth.geojson");
    log::info!(
        "Writing ground truth edges to GeoJSON to {:?}",
        &geojson_dump_filepath
    );
    geofile::geojson::write_lines_to_geojson(&ground_truth_ways, &geojson_dump_filepath)?;
    let topo_result = calculate_topo(
        &proposal_ways,
        &ground_truth_ways,
        &TopoParams {
            resampling_distance: 11.0,
            hole_radius: 7.0,
        },
    )?;
    log::info!("{:?}", topo_result);
    Ok(())
}

fn main() {
    env_logger::init();
    if let Err(e) = try_main() {
        eprintln!("Error: {:?}", e);
        std::process::exit(1)
    }
}
