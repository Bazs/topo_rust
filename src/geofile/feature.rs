use std::collections::HashMap;

pub type FeatureMap = HashMap<String, gdal::vector::FieldValue>;

#[derive(Debug, PartialEq)]
pub struct Feature {
    pub geometry: geo::Geometry,
    pub attributes: Option<FeatureMap>,
}

impl From<geo::Geometry> for Feature {
    fn from(value: geo::Geometry) -> Self {
        Self {
            geometry: value,
            attributes: None,
        }
    }
}
