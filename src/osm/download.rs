extern crate osm_xml as osm;
use anyhow::{anyhow, Ok};
use geohash::{encode, Coord};
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Deserialize, Debug)]
pub struct WgsBoundingBox {
    pub left_lon: f64,
    pub right_lon: f64,
    pub bottom_lat: f64,
    pub top_lat: f64,
}

pub fn get_filename_for_bbox(bbox: &WgsBoundingBox) -> anyhow::Result<String> {
    const GEOHASH_LENGTH: usize = 8;
    let top_left_coord = Coord {
        x: bbox.left_lon,
        y: bbox.top_lat,
    };
    let bottom_right_coord = Coord {
        x: bbox.right_lon,
        y: bbox.bottom_lat,
    };
    let top_left_geohash = encode(top_left_coord, GEOHASH_LENGTH)?;
    let bottom_right_geohash = encode(bottom_right_coord, GEOHASH_LENGTH)?;
    Ok(format!("{top_left_geohash}_{bottom_right_geohash}_osm.xml"))
}

pub fn download_osm_data_by_bbox(bbox: &WgsBoundingBox) -> anyhow::Result<String> {
    let query = format!(
        "https://overpass-api.de/api/map?bbox={},{},{},{}",
        bbox.left_lon, bbox.bottom_lat, bbox.right_lon, bbox.top_lat
    );
    let client = reqwest::blocking::Client::builder()
        .user_agent("osm-geo-mapper")
        .build()?;
    let response = client.get(&query).send()?;
    response.text().or(Err(anyhow!("No response text")))
}

pub fn sync_osm_data_to_file(bbox: &WgsBoundingBox, output_dir: &Path) -> anyhow::Result<PathBuf> {
    let filename = get_filename_for_bbox(bbox)?;
    let output_filepath = output_dir.join(filename);
    if output_filepath.exists() {
        log::info!(
            "Local file exists for OSM data: {:?}",
            output_filepath.canonicalize()
        );
        return Ok(output_filepath);
    }

    log::info!("Downloading OSM data");
    let osm_data = download_osm_data_by_bbox(bbox)?;
    fs::write(&output_filepath, osm_data).or(Err(anyhow!("Could not write OSM data to file")))?;
    Ok(output_filepath)
}


