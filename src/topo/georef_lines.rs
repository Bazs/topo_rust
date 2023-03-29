use std::path::PathBuf;

use anyhow::anyhow;
use geo::LineString;
use proj::Transform;

use crate::{
    crs::crs_utils::{epsg_code_to_authority_string, query_utm_crs_info},
    geofile::gdal_geofile::read_features_from_geofile,
};

// TODO remove this struct in favor of a GeoFeatureGraph
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

pub fn get_utm_zone_for_lines(
    georeferenced_lines: &GeoreferencedLines,
) -> anyhow::Result<gdal::spatial_ref::SpatialRef> {
    if !georeferenced_lines.spatial_ref.is_geographic() {
        return Err(anyhow!("The lines are not in a geographic CRS."));
    }
    match georeferenced_lines.lines.iter().nth(0) {
        Some(line) => match line.coords().nth(0) {
            Some(coord) => {
                let utm_zone_codes = query_utm_crs_info(coord.x, coord.y, Some("WGS84"))?;
                let utm_zone_code = utm_zone_codes
                    .get(0)
                    .ok_or_else(|| (anyhow!("No UTM zones found")))?;
                gdal::spatial_ref::SpatialRef::from_epsg(*utm_zone_code)
                    .map_err(|err| anyhow!("Could not create SpatialRef from EPSG code. {}", err))
            }
            None => {
                return Err(anyhow!(
                    "Could not determine UTM zone for ground truth lines"
                ))
            }
        },
        None => {
            return Err(anyhow!(
                "Could not determine UTM zone for ground truth lines"
            ))
        }
    }
}

pub fn project_lines(
    georeferenced_lines: &GeoreferencedLines,
    to_crs: &gdal::spatial_ref::SpatialRef,
) -> anyhow::Result<GeoreferencedLines> {
    let projection = proj::Proj::new_known_crs(
        &epsg_code_to_authority_string(georeferenced_lines.spatial_ref.auth_code()? as u32),
        &epsg_code_to_authority_string(to_crs.auth_code()? as u32),
        None,
    )?;
    let transformed_lines: anyhow::Result<Vec<LineString>> = georeferenced_lines
        .lines
        .iter()
        .map(|line| {
            line.transformed(&projection)
                .map_err(|err| anyhow!("Could not project line, {}", err))
        })
        .collect();
    Ok(GeoreferencedLines {
        lines: transformed_lines?,
        spatial_ref: to_crs.clone(),
    })
}
