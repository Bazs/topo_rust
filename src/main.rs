extern crate log;
pub mod crs;
pub mod geofile;
pub mod osm;
pub mod topo;
use crate::geofile::feature::Feature;
use crate::geofile::gdal_geofile::{write_features_to_geofile, GdalDriverType};
use crate::osm::download::{sync_osm_data_to_file, WgsBoundingBox};
use crate::topo::georef_lines::{read_lines_from_geofile, GeoreferencedLines};
use crate::topo::topo::{calculate_topo, TopoParams};
use anyhow::anyhow;
use clap::Parser;
use rayon::prelude::*;
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
    Geofile { filepath: PathBuf },
    Osm { bounding_box: WgsBoundingBox },
}

#[derive(Deserialize, Debug)]
struct Config {
    proposal_geofile_path: PathBuf,
    ground_truth: GroundTruthConfig,
    data_dir: PathBuf,
}

fn get_ground_truth_ways_from_osm(
    bounding_box: &WgsBoundingBox,
    data_dir: &PathBuf,
) -> anyhow::Result<Vec<geo::LineString>> {
    log::info!("Syncing OSM data for bounding box {:?}", bounding_box);
    let osm_filepath = sync_osm_data_to_file(&bounding_box, &data_dir)?;
    log::info!("Reading OSM ways");
    osm::conversion::read_osm_roads_from_file(&osm_filepath)
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

    let mut ground_truth_georef_lines = match config.ground_truth {
        GroundTruthConfig::Osm { bounding_box } => {
            let ground_truth_ways =
                get_ground_truth_ways_from_osm(&bounding_box, &config.data_dir)?;
            GeoreferencedLines {
                lines: ground_truth_ways,
                spatial_ref: gdal::spatial_ref::SpatialRef::from_epsg(4326).unwrap(),
            }
        }
        GroundTruthConfig::Geofile { filepath } => read_lines_from_geofile(&filepath)?,
    };
    log::info!(
        "Read {} ground truth edges",
        ground_truth_georef_lines.lines.len()
    );

    let mut proposal_georef_lines = read_lines_from_geofile(&config.proposal_geofile_path)?;
    log::info!("Read {} proposal edges", proposal_georef_lines.lines.len());
    let geojson_dump_filepath = config.data_dir.join("ground_truth.geojson");

    // Write the ground truth to file for reference.
    log::info!(
        "Writing ground truth edges to GeoJSON to {:?}",
        &geojson_dump_filepath
    );
    geofile::geojson::write_lines_to_geojson(
        &ground_truth_georef_lines.lines,
        &geojson_dump_filepath,
    )?;

    topo::preprocessing::ensure_gt_proposal_same_projected_crs(
        &mut ground_truth_georef_lines,
        &mut proposal_georef_lines,
    )?;

    let topo_result = calculate_topo(
        &proposal_georef_lines.lines,
        &ground_truth_georef_lines.lines,
        &TopoParams {
            resampling_distance: 11.0,
            hole_radius: 7.0,
        },
    )?;
    log::info!("{:?}", topo_result.f1_score_result);
    write_features_to_geofile(
        &topo_result
            .proposal_nodes
            .par_iter()
            .map(|node| Feature::from(node))
            .collect(),
        &config.data_dir.join("proposal_nodes.gpkg"),
        Some(&proposal_georef_lines.spatial_ref),
        GdalDriverType::GeoPackage.name(),
    )?;
    write_features_to_geofile(
        &topo_result
            .ground_truth_nodes
            .par_iter()
            .map(|node| Feature::from(node))
            .collect(),
        &config.data_dir.join("ground_truth_nodes.gpkg"),
        Some(&ground_truth_georef_lines.spatial_ref),
        GdalDriverType::GeoPackage.name(),
    )?;
    Ok(())
}

fn main() {
    env_logger::init();
    if let Err(e) = try_main() {
        eprintln!("Error: {:?}", e);
        std::process::exit(1)
    }
}
