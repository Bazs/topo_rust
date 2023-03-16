use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub struct Feature {
    pub geometry: geo::Geometry,
    // TODO support different value types besides String. See gdal::vector::OGRFieldType for types
    // supported by GDAL.
    pub attributes: Option<HashMap<String, gdal::vector::FieldValue>>,
}

impl From<geo::Geometry> for Feature {
    fn from(value: geo::Geometry) -> Self {
        Self {
            geometry: value,
            attributes: None,
        }
    }
}
