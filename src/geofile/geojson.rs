use std::{fs, io, path::Path};

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
