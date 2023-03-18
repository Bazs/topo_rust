use std::path::PathBuf;

use super::gdal_geofile::read_features_from_geofile;

pub struct GeoreferencedLines {
    pub lines: Vec<geo::LineString>,
    pub spatial_ref: gdal::spatial_ref::SpatialRef,
}

pub fn read_lines_from_geofile(filepath: &PathBuf) -> anyhow::Result<GeoreferencedLines> {
    let (features, spatial_ref) = read_features_from_geofile(filepath)?;
    let num_features = features.len();
    let lines: Vec<geo::LineString> = features
        .into_iter()
        .filter_map(|feature| match feature.geometry {
            geo::Geometry::LineString(linestring) => Some(linestring),
            _ => None,
        })
        .collect();
    if lines.len() != num_features {
        log::warn!(
            "Out of {} features read, only {} were LineStrings.",
            num_features,
            lines.len()
        )
    }
    Ok(GeoreferencedLines { lines, spatial_ref })
}
