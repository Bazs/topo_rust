use std::cmp::Ordering;

use anyhow::anyhow;
use geo::LineString;
use utm::{lat_lon_to_zone_number, lat_to_zone_letter, to_utm_wgs84};

pub fn utm_zone_to_crs(
    zone_number: u8,
    zone_letter: char,
    datum: Option<&str>,
) -> anyhow::Result<gdal::spatial_ref::SpatialRef> {
    const EQUATOR_ZONE_LETTER: char = 'M';
    const EQUATOR_ZONE_LETTER_INT: i32 = EQUATOR_ZONE_LETTER as i32;
    const MAX_VALID_ZONE_LETTER_INT: i32 = 'X' as i32;
    if zone_letter as i32 > MAX_VALID_ZONE_LETTER_INT {
        return Err(anyhow!("Invalid zone letter {}", zone_letter));
    }
    let zone_letter_int = zone_letter as i32 - EQUATOR_ZONE_LETTER_INT;
    let north_or_south = match zone_letter_int.cmp(&0) {
        Ordering::Equal | Ordering::Less => "+south",
        Ordering::Greater => "",
    };

    let datum = datum.unwrap_or("WGS84");
    let proj4_definition = format!(
        "+proj=utm +zone={} {} +datum={}",
        zone_number, north_or_south, datum
    );
    log::debug!(
        "Using proj4 WKT for UTM zone {}{}: {}",
        zone_number,
        zone_letter,
        proj4_definition
    );

    let mut spatial_ref = gdal::spatial_ref::SpatialRef::from_proj4(proj4_definition.as_str())
        .or_else(|err| Err(anyhow!("Could not determine UTM CRS: {}", err)))?;

    if spatial_ref.auto_identify_epsg().is_err() {
        log::debug!(
            "Could not identify EPSG info for CRS {:?}",
            spatial_ref.to_wkt()?
        );
    };

    Ok(spatial_ref)
}

pub fn get_utm_zone_for_wgs84_lines(wgs84_lines: &Vec<LineString>) -> anyhow::Result<(u8, char)> {
    match wgs84_lines.iter().nth(0) {
        Some(line) => match line.coords().nth(0) {
            Some(coord) => match lat_to_zone_letter(coord.y) {
                Some(zone_letter) => Ok((lat_lon_to_zone_number(coord.y, coord.x), zone_letter)),
                None => Err(anyhow!(
                    "Could not determine UTM zone letter for latitude{}",
                    coord.y
                )),
            },
            None => Err(anyhow!(
                "Could not determine UTM zone for ground truth lines"
            )),
        },
        None => Err(anyhow!(
            "Could not determine UTM zone for ground truth lines"
        )),
    }
}

pub fn convert_wgs84_lines_to_utm(
    wgs84_lines: &Vec<LineString>,
    utm_zone_number: u8,
) -> Vec<LineString> {
    wgs84_lines
        .iter()
        .map(|line| {
            line.coords()
                .map(|coord| {
                    let (northing, easting, _) = to_utm_wgs84(coord.y, coord.x, utm_zone_number);
                    (easting, northing)
                })
                .collect()
        })
        .collect()
}
