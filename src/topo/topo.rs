use std::borrow::Borrow;

use geo::{EuclideanLength, HasDimensions};

pub struct TopoResult {
    precision: f64,
    recall: f64,
    f1_score: f64,
}

pub struct TopoParams {
    pub resampling_distance: f64,
}

pub fn calculate_topo(
    proposal: &Vec<geo::LineString>,
    ground_truth: &Vec<geo::LineString>,
    params: &TopoParams,
) -> anyhow::Result<TopoResult> {
    // Interpolate the edges.

    unimplemented!();
}

fn resample_line(linestr: &geo::LineString, resampling_distance: f64) -> Option<geo::LineString> {
    if linestr.is_empty() {
        return None;
    }

    // Calculate equidistant split points maintaining the same number of splits.
    let linestr_len = linestr.euclidean_length();
    let num_parts = (linestr_len / resampling_distance) as i64;
    let resampling_distance = linestr_len / num_parts as f64;

    let mut output_coords = vec![*linestr.coords().nth(0).unwrap()];
    let mut prev_inserted_dist = 0.0;
    let mut prev_original_vertex_dist = 0.0;
    let mut next_original_vert_dist = 0.0;
    for line in linestr.lines() {
        let line_len = line.euclidean_length();
        next_original_vert_dist += line_len;
        while (next_original_vert_dist - prev_inserted_dist) < resampling_distance {
            let new_insert_dist = prev_inserted_dist + resampling_distance;
            let new_coord = line.start * (next_original_vert_dist - new_insert_dist) / line_len
                + line.end * (new_insert_dist - prev_original_vertex_dist) / line_len;
            output_coords.push(new_coord);
            prev_inserted_dist = new_insert_dist;
        }
        prev_original_vertex_dist = next_original_vert_dist;
    }
    output_coords.push(*linestr.coords().last().unwrap());
    Some(output_coords.into_iter().collect())
}
