use std::{collections::HashMap, path::PathBuf};

use crate::{
    geofile::{
        feature::{Feature, FeatureMap},
        gdal_geofile::read_features_from_geofile,
    },
    geograph,
};

use super::primitives::GeoGraph;

/// A GeoGraph whose edge and node data type is a FeatureMap. Can be constructed from features read from a geofile.
pub type GeoFeatureGraph<Ty> = GeoGraph<FeatureMap, FeatureMap, Ty>;

impl<Ty: petgraph::EdgeType> TryFrom<Vec<Feature>> for GeoFeatureGraph<Ty> {
    type Error = anyhow::Error;

    fn try_from(features: Vec<Feature>) -> anyhow::Result<Self> {
        let num_features = features.len();
        let (lines, data): (Vec<geo::LineString>, Vec<FeatureMap>) = features
            .into_iter()
            .filter_map(|feature| match feature.geometry {
                geo::Geometry::LineString(linestring) => {
                    Some((linestring, feature.attributes.unwrap_or_else(HashMap::new)))
                }
                _ => None,
            })
            .unzip();
        if lines.len() != num_features {
            log::warn!(
                "Out of {} features read, only {} were LineStrings.",
                num_features,
                lines.len()
            )
        }
        geograph::utils::build_geograph_from_lines_with_data(lines, data)
    }
}

impl<Ty: petgraph::EdgeType> GeoFeatureGraph<Ty> {
    pub fn load_from_geofile(filepath: &PathBuf) -> anyhow::Result<Self> {
        let (features, spatial_ref) = read_features_from_geofile(filepath)?;
        let mut graph: GeoFeatureGraph<Ty> = features.try_into()?;
        graph.crs = spatial_ref;
        Ok(graph)
    }
}
