use anyhow::anyhow;
use geo::LineString;
use utm::{lat_lon_to_zone_number, to_utm_wgs84};

pub fn get_utm_zone_number_for_wgs84_lines(wgs84_lines: &Vec<LineString>) -> anyhow::Result<u8> {
    match wgs84_lines.iter().nth(0) {
        Some(line) => match line.coords().nth(0) {
            Some(coord) => Ok(lat_lon_to_zone_number(coord.y, coord.x)),
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
                    let (easting, northing, _) = to_utm_wgs84(coord.y, coord.x, utm_zone_number);
                    (easting, northing)
                })
                .collect()
        })
        .collect()
}
