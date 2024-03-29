extern crate log;
pub mod crs;
pub mod geofile;
pub mod geograph;
pub mod osm;
pub mod topo;
use crate::crs::crs_utils::epsg_4326;
use crate::geofile::feature::Feature;
use crate::geofile::gdal_geofile::{write_features_to_geofile, GdalDriverType};
use crate::geograph::geo_feature_graph::GeoFeatureGraph;
use crate::geograph::utils::build_geograph_from_lines;
use crate::osm::download::{sync_osm_data_to_file, WgsBoundingBox};
use crate::topo::topo::{calculate_topo, TopoParams};
use anyhow::anyhow;
use clap::Parser;
use rayon::prelude::*;
use serde::Deserialize;
use std::path::PathBuf;
use std::{fs::read_to_string, path::Path};

/// Calculate the TOPO metric over a ground truth and a proposal road map.
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
    topo_params: TopoParams,
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

    let mut ground_truth_graph: GeoFeatureGraph<petgraph::Undirected> = match config.ground_truth {
        GroundTruthConfig::Osm { bounding_box } => {
            let ground_truth_ways =
                get_ground_truth_ways_from_osm(&bounding_box, &config.data_dir)?;
            let mut graph = build_geograph_from_lines(ground_truth_ways)?;
            graph.crs = epsg_4326();
            graph
        }
        GroundTruthConfig::Geofile { filepath } => GeoFeatureGraph::load_from_geofile(&filepath)?,
    };
    log::info!(
        "Read ground truth graph with {}  edges",
        ground_truth_graph.edge_graph().edge_count()
    );

    let mut proposal_graph = GeoFeatureGraph::load_from_geofile(&config.proposal_geofile_path)?;
    log::info!(
        "Read proposal graph with {} edges",
        proposal_graph.edge_graph().edge_count()
    );
    let geojson_dump_filepath = config.data_dir.join("ground_truth.geojson");

    // Write the ground truth to file for reference.
    log::info!(
        "Writing ground truth edges to GeoJSON to {:?}",
        &geojson_dump_filepath
    );
    geofile::geojson::write_lines_to_geojson(
        &ground_truth_graph.edge_geometries(),
        &geojson_dump_filepath,
    )?;

    topo::preprocessing::ensure_gt_proposal_in_same_projected_crs(
        &mut ground_truth_graph,
        &mut proposal_graph,
    )?;

    let topo_result = calculate_topo(&proposal_graph, &ground_truth_graph, &config.topo_params)?;
    log::info!("{:?}", topo_result.f1_score_result);
    write_features_to_geofile(
        &topo_result
            .proposal_nodes
            .par_iter()
            .map(|node| Feature::from(node))
            .collect(),
        &config.data_dir.join("proposal_nodes.gpkg"),
        Some(&proposal_graph.crs),
        GdalDriverType::GeoPackage.name(),
    )?;
    write_features_to_geofile(
        &topo_result
            .ground_truth_nodes
            .par_iter()
            .map(|node| Feature::from(node))
            .collect(),
        &config.data_dir.join("ground_truth_nodes.gpkg"),
        Some(&ground_truth_graph.crs),
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
