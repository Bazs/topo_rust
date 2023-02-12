use anyhow::anyhow;
// use std::env::temp_dir;
// use std::fs::File;
// use std::io::Write;
// use uuid::Uuid;

use geohash::{encode, Coord};

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
    // let result = response.text()?;
    // let mut tempfile = temp_dir();
    // tempfile.push(Uuid::new_v4().to_string());
    // tempfile.set_extension("xml");
    // let mut file = File::create(&tempfile)?;
    // write!(file, "{}", result)?;
    // Ok(tempfile.as_path().to_str().unwrap().to_string())
}
