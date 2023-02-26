use std::{
    fs::{self, read_to_string},
    io,
    path::{Path, PathBuf},
};

use anyhow::anyhow;

pub fn write_lines_to_geojson(
    lines: &Vec<geo::LineString>,
    output_filepath: &Path,
) -> io::Result<()> {
    let feature_collection: geojson::FeatureCollection = lines
        .iter()
        .map(|line| geojson::Feature::from(geojson::Geometry::from(line)))
        .collect();
    let geojson_contents: geojson::GeoJson = geojson::GeoJson::from(feature_collection);
    fs::write(output_filepath, geojson_contents.to_string())
}

pub fn read_lines_from_geojson(filepath: &PathBuf) -> anyhow::Result<Vec<geo::LineString>> {
    let geojson_contents = read_to_string(filepath)?;
    let feature_collection = geojson_contents.parse::<geojson::FeatureCollection>()?;
    let lines: Result<Vec<_>, _> = feature_collection
        .into_iter()
        .map(|feature| geo::LineString::try_from(feature))
        .collect();
    lines.or_else(|error| Err(anyhow!("Could not parse linestrings, {}", error)))
}
