extern crate osm_xml as osm;
use anyhow::anyhow;
use std::{borrow::Borrow, path::Path};

pub fn read_osm_ways_from_file(filepath: &Path) -> anyhow::Result<Vec<geo::LineString>> {
    let infile = std::fs::File::open(filepath)?;
    let data = osm::OSM::parse(infile)?;
    data.ways
        .borrow()
        .into_iter()
        .map(|(_, way)| osm_way_to_linestring(&data, &way))
        .collect()
}

fn osm_way_to_linestring(osm_data: &osm::OSM, way: &osm::Way) -> anyhow::Result<geo::LineString> {
    let mut points: Vec<geo::Point> = Vec::new();
    for node in &way.nodes {
        if let osm::Reference::Node(node) = osm_data.resolve_reference(&node) {
            points.push(geo::Point::new(node.lon, node.lat));
        } else {
            return Err(anyhow!("Expected a node"));
        }
    }
    Ok(points.into_iter().collect())
}
