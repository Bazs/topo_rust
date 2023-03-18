use core::slice;
use libc::c_char;
use proj_sys;
use std::{
    cmp::Ordering,
    ffi::{c_int, CString},
    ptr::null_mut,
    str::from_utf8,
};

use anyhow::anyhow;
use geo::LineString;
use utm::{lat_lon_to_zone_number, lat_to_zone_letter, to_utm_wgs84};

pub fn utm_zone_to_spatial_ref(
    zone_number: u8,
    zone_letter: char,
    datum: Option<&str>,
) -> anyhow::Result<gdal::spatial_ref::SpatialRef> {
    let proj4_definition = utm_zone_to_proj_string(zone_number, zone_letter, datum)?;
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

pub fn utm_zone_to_proj_string(
    zone_number: u8,
    zone_letter: char,
    datum: Option<&str>,
) -> anyhow::Result<String> {
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
    Ok(format!(
        "+proj=utm +zone={} {} +datum={}",
        zone_number, north_or_south, datum
    ))
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

/// Query UTM zones which contain the lon/lat WGS84 coordinate.
///
/// # Arguments
/// * lon - longitude in degrees.
/// * lat - latitude in degrees.
/// * datum_name - the name of the geodetic datum to query for. Example: "WGS84", "NAD83". If not specified, zones
///     with all datums are returned.
///
/// # Returns
/// Authority codes of any found UTM zones, e.g. "EPSG:32654".
pub fn query_utm_crs_info(
    lon: f64,
    lat: f64,
    datum_name: Option<&str>,
) -> anyhow::Result<Vec<String>> {
    let mut results = Vec::new();
    unsafe {
        let context = proj_sys::proj_context_create();
        let auth_name = CString::new("EPSG").unwrap();
        let crs_types: [proj_sys::PJ_TYPE; 1] = [proj_sys::PJ_TYPE_PJ_TYPE_PROJECTED_CRS];
        let query_params = proj_sys::proj_get_crs_list_parameters_create();
        (*query_params).types = crs_types.as_ptr();
        (*query_params).typesCount = 1;

        (*query_params).bbox_valid = true as i32;
        (*query_params).west_lon_degree = lon;
        (*query_params).south_lat_degree = lat;
        (*query_params).east_lon_degree = lon;
        (*query_params).north_lat_degree = lat;

        let out_result_count: *mut c_int = null_mut();

        let mut crs_info_list = proj_sys::proj_get_crs_info_list_from_database(
            context,
            auth_name.as_ptr(),
            query_params,
            out_result_count,
        );
        // Store the pointer returned by proj_get_crs_info_list_from_database to destroy it later with proj_crs_info_list_destroy.
        let crs_info_list_original = crs_info_list;

        proj_sys::proj_get_crs_list_parameters_destroy(query_params);
        proj_sys::proj_context_destroy(context);

        if crs_info_list.is_null() {
            return Err(anyhow!("Failed to query UTM zones."));
        }

        while !(*crs_info_list).is_null() {
            let crs_info = **crs_info_list;
            crs_info_list = crs_info_list.offset(1);

            let crs_name = i8_ptr_as_str(crs_info.name)?;
            if !crs_name.contains("UTM zone") {
                continue;
            }
            if let Some(datum_name) = datum_name {
                // UTM zone names start with the datum name as e.g. "WGS 87 / UTM zone ..."
                // Split out the datum name and remvove the spaces.
                let crs_datum = crs_name
                    .split("/")
                    .nth(0)
                    .ok_or_else(|| anyhow!("CRS '{}' does not have a datum specifier", crs_name))?;
                let crs_datum = crs_datum.replace(" ", "");
                if crs_datum != datum_name {
                    continue;
                }
            }
            let auth_name = i8_ptr_as_str(crs_info.auth_name)?;
            let auth_code = i8_ptr_as_str(crs_info.code)?;
            results.push(format!("{}:{}", auth_name, auth_code));
        }
        proj_sys::proj_crs_info_list_destroy(crs_info_list_original);
    }
    Ok(results)
}

fn i8_ptr_as_str(c_string: *const c_char) -> anyhow::Result<&'static str> {
    unsafe {
        let slice = slice::from_raw_parts(
            c_string as *const u8,
            libc::strlen(c_string as *const c_char),
        );
        from_utf8(slice).or_else(|err| Err(anyhow!("Could not decode string {}", err)))
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

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use std::collections::HashSet;

    use crate::crs::utm_conversion::query_utm_crs_info;

    #[rstest]
    #[case(139.813385, 35.707317999, Some("WGS84"), vec!("EPSG:32654"))] // WGS 84 UTM zone 54N for a coordinate in Tokyo.
    #[case(139.813385, 35.707317999, Some("Tokyo"), vec!("EPSG:3095"))] // UTM zone 54N in the "Tokyo" projection (because of course that exists).
    #[case(139.813385, 35.707317999, Some("NAD83"), vec!())] // NAD 83 is not defined in Japan.
    #[case(-98.261719, 35.581384, Some("NAD83"), vec!("EPSG:26914"))] // NAD 83 UMT zone 14N for a coordinate in the US.
    fn test_query_utm_crs_info(
        #[case] lon: f64,
        #[case] lat: f64,
        #[case] datum_name: Option<&str>,
        #[case] expected_results: Vec<&str>,
    ) {
        let results = query_utm_crs_info(lon, lat, datum_name).unwrap();
        let results_set: HashSet<&str> = results.iter().map(std::ops::Deref::deref).collect();
        let expected_results_set: HashSet<&str> = expected_results.into_iter().collect();
        assert_eq!(results_set, expected_results_set);
    }
}
