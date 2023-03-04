use anyhow::anyhow;
use gdal::vector::LayerAccess;
use std::path::Path;

pub fn write_lines_to_geopackage(
    lines: &Vec<geo::LineString>,
    output_filepath: &Path,
    crs: Option<gdal::spatial_ref::SpatialRef>,
) -> anyhow::Result<()> {
    let crs = crs.unwrap_or(gdal::spatial_ref::SpatialRef::from_epsg(4326).unwrap());
    let crs_name = crs.name()?;
    log::debug!("Using spatial ref {} for writing geofile", crs_name);

    let driver = gdal::DriverManager::get_driver_by_name("GPKG")?;
    let mut dataset = driver.create_vector_only(output_filepath)?;
    let layer_options = gdal::LayerOptions {
        name: "",
        srs: Some(&crs),
        ty: gdal::vector::OGRwkbGeometryType::wkbLineString,
        options: None,
    };

    let mut layer = dataset.create_layer(layer_options)?;
    for line in lines {
        // let mut feature = gdal::vector::Feature::new(layer.defn())?;
        let geo_geometry = geo::Geometry::from(line.to_owned());
        let wkb = wkb::geom_to_wkb(&geo_geometry)
            .or_else(|err| Err(anyhow!("Could not write geometry to WKB")))?;
        let geometry = gdal::vector::Geometry::from_wkb(&wkb)?;
        // feature.set_geometry(geometry)?;
        layer.create_feature(geometry)?;
    }
    Ok(())
}
