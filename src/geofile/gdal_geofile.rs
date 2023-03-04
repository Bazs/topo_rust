use anyhow::{anyhow, Context};
use gdal::vector::LayerAccess;
use rayon::prelude::*;
use std::{collections::HashSet, path::Path};

use super::feature::Feature;

pub enum GdalDriverType {
    GeoPackage,
    GeoJson,
}

impl GdalDriverType {
    pub fn name(&self) -> &'static str {
        match self {
            GdalDriverType::GeoPackage => "GPKG",
            GdalDriverType::GeoJson => "GeoJSON",
        }
    }
}

pub fn write_features_to_geofile(
    features: &Vec<Feature>,
    output_filepath: &Path,
    crs: Option<&gdal::spatial_ref::SpatialRef>,
    // TODO make driver optional and attempt to derive it from extension
    driver: &str,
) -> anyhow::Result<()> {
    let driver = gdal::DriverManager::get_driver_by_name(driver).context("Getting GDAL driver")?;

    if features.is_empty() {
        return Ok(());
    }
    let layer_type = {
        use gdal::vector::OGRwkbGeometryType::*;
        let geometry = &features.iter().nth(0).unwrap().geometry;
        match geometry {
            geo::Geometry::Point(_) => wkbPoint,
            geo::Geometry::LineString(_) => wkbLineString,
            geo::Geometry::Polygon(_) => wkbPolygon,
            geo::Geometry::MultiPoint(_) => wkbMultiPoint,
            geo::Geometry::MultiLineString(_) => wkbMultiLineString,
            geo::Geometry::MultiPolygon(_) => wkbMultiPolygon,
            _ => {
                return Err(anyhow!("Cannot write geometry type {:?} to file.", {
                    geometry
                }))
            }
        }
    };

    let crs = match crs {
        Some(crs) => crs.clone(),
        None => gdal::spatial_ref::SpatialRef::from_epsg(4326).unwrap(),
    };
    let crs_name = crs.name()?;
    log::debug!("Using spatial ref {} for writing geofile", crs_name);

    let mut dataset = driver.create_vector_only(output_filepath)?;
    let layer_options = gdal::LayerOptions {
        name: "",
        srs: Some(&crs),
        ty: layer_type,
        options: None,
    };

    let mut layer = dataset.create_layer(layer_options)?;

    // Create the fields based on all attributes of all features.
    let field_names = get_field_names(features);
    let field_definitions: Vec<(&str, gdal::vector::OGRFieldType::Type)> = field_names
        .iter()
        .map(|field_name| (field_name as &str, gdal::vector::OGRFieldType::OFTString))
        .collect();
    layer.create_defn_fields(&field_definitions)?;

    log::info!(
        "Writing {} features to {:?}",
        features.len(),
        output_filepath
    );

    for feature in features {
        let wkb = wkb::geom_to_wkb(&feature.geometry)
            .or_else(|err| Err(anyhow!("Could not write geometry to WKB, {:?}", err)))?;
        let geometry = gdal::vector::Geometry::from_wkb(&wkb)?;

        match &feature.attributes {
            Some(attributes) => {
                let mut field_names = Vec::new();
                let mut values = Vec::new();
                for (key, value) in attributes {
                    field_names.push(key);
                    values.push(gdal::vector::FieldValue::StringValue(value.to_owned()))
                }
                let field_names: Vec<&str> = field_names.iter().map(|name| name as &str).collect();
                layer.create_feature_fields(geometry, &field_names, &values)?;
            }
            None => layer.create_feature(geometry)?,
        }
    }
    Ok(())
}

fn get_field_names(features: &Vec<Feature>) -> Vec<String> {
    let fields: HashSet<String> = features
        .par_iter()
        .filter_map(|feature| match &feature.attributes {
            Some(attributes) => Some(attributes.keys().cloned().collect::<Vec<String>>()),
            None => None,
        })
        .flatten()
        .collect();
    fields.into_iter().collect()
}
