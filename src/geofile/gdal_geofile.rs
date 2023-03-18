use anyhow::{anyhow, Context};
use gdal::vector::FieldValue;
use gdal::vector::LayerAccess;
use indicatif::ProgressBar;
use rayon::prelude::*;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

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

/// Write features to a geofile.
///
/// # Arguments
/// * features - The features to write. NOTE: all features will be written as string regardless of their type.
/// * crs - The CRS to set for the geofile. Defaults to EPSG:4326 if None.
/// * driver - Name of the GDAL driver to use. GdalDriverType has some options.
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
        // TODO verify that all features have the same geometry type up front.
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
        None => get_default_spatial_ref(),
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
    log::info!("Setting up fields");
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
    unsafe {
        // Start a transaction in case the driver supports transactions, e.g. GeoPackage.
        // Committing all features once as opposed to per-feature is a massive speedup for these drivers.
        gdal_sys::OGR_L_StartTransaction(layer.c_layer());
    };
    let bar = ProgressBar::new(features.len() as u64);
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
                    values.push(value.to_owned())
                }
                let field_names: Vec<&str> = field_names.iter().map(|name| name as &str).collect();
                layer.create_feature_fields(geometry, &field_names, &values)?;
            }
            None => layer.create_feature(geometry)?,
        }

        bar.inc(1);
    }
    unsafe {
        // Start a transaction in case the driver supports transactions.
        gdal_sys::OGR_L_CommitTransaction(layer.c_layer());
    };
    Ok(())
}

pub fn read_features_from_geofile(
    filepath: &Path,
) -> anyhow::Result<(Vec<Feature>, gdal::spatial_ref::SpatialRef)> {
    gdal::DriverManager::register_all();
    let mut open_options = gdal::DatasetOptions::default();
    open_options.open_flags = gdal::GdalOpenFlags::GDAL_OF_VECTOR;
    let dataset = gdal::Dataset::open_ex(filepath, open_options)?;

    let layer_count = dataset.layer_count();
    if 0 == layer_count || 1 < layer_count {
        // Note: in principle any amount of layers could be read in a loop, their features combined into one collection. Implement if necessary.
        return Err(anyhow!(
            "Found {} layers, only one layer is supported.",
            layer_count
        ));
    }
    let mut layer = dataset.layer(0)?;

    let mut features = Vec::new();
    features.reserve(layer.feature_count() as usize);

    log::info!("Reading {} features", layer.feature_count());

    for gdal_feature in layer.features() {
        let attributes: HashMap<String, FieldValue> = gdal_feature
            .fields()
            .into_iter()
            .filter_map(|(field_name, field_value)| {
                if let Some(value) = field_value {
                    return Some((field_name, value));
                }
                return None;
            })
            .collect();
        let wkb = gdal_feature.geometry().wkb()?;
        let geometry = wkb::wkb_to_geom(&mut wkb.as_slice())
            .or_else(|err| Err(anyhow!("Could not parse geometry from WKB, {:?}", err)))?;
        let attributes = if attributes.is_empty() {
            None
        } else {
            Some(attributes)
        };

        features.push(Feature {
            geometry: geometry,
            attributes: attributes,
        });
    }

    let spatial_ref = layer.spatial_ref().unwrap_or(get_default_spatial_ref());

    return Ok((features, spatial_ref));
}

fn get_default_spatial_ref() -> gdal::spatial_ref::SpatialRef {
    gdal::spatial_ref::SpatialRef::from_epsg(4326).unwrap()
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

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, iter::zip};

    use gdal::vector::FieldValue;
    use rstest::rstest;
    use testdir::testdir;

    use crate::geofile::{
        feature::Feature,
        gdal_geofile::{read_features_from_geofile, write_features_to_geofile, GdalDriverType},
    };

    #[rstest]
    #[case(GdalDriverType::GeoJson)]
    #[case(GdalDriverType::GeoPackage)]
    fn test_geofile_write_read_round_trip(#[case] driver: GdalDriverType) {
        let features = vec![Feature {
            geometry: geo::Geometry::Point(geo::Point::new(80.0, 45.0)),
            attributes: Some(HashMap::from([
                (
                    "key1".to_string(),
                    FieldValue::StringValue("value1".to_string()),
                ),
                (
                    "key2".to_string(),
                    FieldValue::StringValue("56.0".to_string()),
                ),
            ])),
        }];

        let test_dir = testdir!();
        let geofile_filepath = test_dir.join("output.file");

        let spatial_ref = gdal::spatial_ref::SpatialRef::from_epsg(4326).unwrap();

        write_features_to_geofile(
            &features,
            &geofile_filepath,
            Some(&spatial_ref),
            driver.name(),
        )
        .unwrap();
        let (read_features, read_spatial_ref) =
            read_features_from_geofile(&geofile_filepath).unwrap();

        for (feature, read_feature) in zip(features, read_features) {
            assert_eq!(feature, read_feature);
        }
        let read_spatial_ref_name = read_spatial_ref.name().unwrap();
        let spatial_ref_name = spatial_ref.name().unwrap();
        assert_eq!(read_spatial_ref_name, spatial_ref_name);
    }
}
