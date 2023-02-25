use std::{collections::HashSet, f64::consts::FRAC_PI_2};

use geo::{CoordsIter, EuclideanLength};
use kdtree::distance::squared_euclidean;
use rayon::prelude::*;

pub struct TopoResult {
    precision: f64,
    recall: f64,
    f1_score: f64,
}

pub struct TopoParams {
    pub resampling_distance: f64,
    pub hole_radius: f64,
}

pub fn calculate_topo(
    proposal: &Vec<geo::LineString>,
    ground_truth: &Vec<geo::LineString>,
    params: &TopoParams,
) -> anyhow::Result<TopoResult> {
    // Interpolate the edges.
    let proposal_points = sample_points_on_lines(proposal, params.resampling_distance);
    let mut proposal_nodes = road_points_to_topo_nodes(&proposal_points);
    let ground_truth_points: Vec<RoadPoint> =
        sample_points_on_lines(ground_truth, params.resampling_distance);
    let mut ground_truth_nodes = road_points_to_topo_nodes(&ground_truth_points);
    let ground_truth_kdtree = build_kdtree_from_nodes(&ground_truth_nodes)?;

    let mut matched_gt_ids = HashSet::new();

    // TODO use par_iter_mut to parallelize
    for mut proposal_node in proposal_nodes {
        // TODO implement matching also based on azimuth
        let gt_nodes_within_range = ground_truth_kdtree.within(
            &<[f64; 2]>::from(proposal_node.road_point.coord),
            params.hole_radius,
            &squared_euclidean,
        )?;
        for gt_node in gt_nodes_within_range {
            if !matched_gt_ids.contains(gt_node.1) {
                proposal_node.matched = true;
                // TODO implement saving the match distance proposal_node.match_distance =
                matched_gt_ids.insert(gt_node.1);
            }
        }
    }
    let matched_count = matched_gt_ids.len();
    // TODO calculate score
    unimplemented!();
}

struct RoadPoint {
    coord: geo::Coord,
    azimuth: f64,
}

struct TopoNode<'a> {
    road_point: &'a RoadPoint,
    id: i64,
    matched: bool,
    match_distance: Option<f64>,
}

impl<'a> TopoNode<'a> {
    fn new(point: &'a RoadPoint, id: i64) -> Self {
        TopoNode {
            road_point: &point,
            id: id,
            matched: false,
            match_distance: None,
        }
    }
}

fn build_kdtree_from_nodes(
    topo_nodes: &Vec<TopoNode>,
) -> anyhow::Result<kdtree::KdTree<f64, i64, [f64; 2]>> {
    let mut kdtree = kdtree::KdTree::with_capacity(2, topo_nodes.len());
    for node in topo_nodes {
        kdtree.add(<[f64; 2]>::from(node.road_point.coord), node.id)?;
    }
    Ok(kdtree)
}

fn road_points_to_topo_nodes(road_points: &Vec<RoadPoint>) -> Vec<TopoNode> {
    road_points
        .iter()
        .enumerate()
        .map(|(idx, road_point)| TopoNode::new(&road_point, idx as i64))
        .collect()
}

fn sample_points_on_lines(
    lines: &Vec<geo::LineString>,
    resampling_distance: f64,
) -> Vec<RoadPoint> {
    lines
        .par_iter()
        .map(|linestr| sample_points_on_line(linestr, resampling_distance))
        .flatten()
        .collect()
}

fn sample_points_on_line(linestr: &geo::LineString, resampling_distance: f64) -> Vec<RoadPoint> {
    if 2 > linestr.coords_count() {
        return vec![];
    }
    if resampling_distance <= 0.0 {
        return vec![];
    }

    // Calculate equidistant split points maintaining the same number of splits.
    let linestr_len = linestr.euclidean_length();
    let num_parts = (linestr_len / resampling_distance) as i64;
    let resampling_distance = linestr_len / num_parts as f64;

    let mut output_points = vec![RoadPoint {
        coord: *linestr.coords().nth(0).unwrap(),
        azimuth: get_normalized_line_azimuth(&linestr.lines().nth(0).unwrap()),
    }];

    let mut prev_inserted_dist = 0.0;
    let mut prev_original_vertex_dist = 0.0;
    let mut next_original_vert_dist = 0.0;
    for line in linestr.lines() {
        let line_len = line.euclidean_length();
        next_original_vert_dist += line_len;
        let mut azimuth: Option<f64> = None;
        while (next_original_vert_dist - prev_inserted_dist) > resampling_distance {
            let azimuth = azimuth.get_or_insert_with(|| get_normalized_line_azimuth(&line));
            let new_insert_dist = prev_inserted_dist + resampling_distance;
            let new_coord = line.start * (next_original_vert_dist - new_insert_dist) / line_len
                + line.end * (new_insert_dist - prev_original_vertex_dist) / line_len;
            output_points.push(RoadPoint {
                coord: new_coord,
                azimuth: *azimuth,
            });
            prev_inserted_dist = new_insert_dist;
        }
        prev_original_vertex_dist = next_original_vert_dist;
    }
    output_points.push(RoadPoint {
        coord: *linestr.coords().last().unwrap(),
        azimuth: get_normalized_line_azimuth(&linestr.lines().last().unwrap()),
    });
    output_points
}

fn get_normalized_line_azimuth(line: &geo::Line) -> f64 {
    let mut delta = line.delta();

    // Normalize the delta so the X component is always positive.
    if delta.x < 0.0 {
        delta = -delta;
    }
    let azimuth = delta.y.atan2(delta.x);
    if azimuth == -FRAC_PI_2 {
        // Treat a vertical upwards line the same as a vertical downwards line.
        return FRAC_PI_2;
    }
    azimuth
}

#[cfg(test)]
mod tests {
    extern crate approx;
    use approx::assert_abs_diff_eq;
    use geo::{Coord, LineString};
    use rstest::rstest;
    use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

    use super::get_normalized_line_azimuth;

    fn tuple_vec_to_linestring(tuple_vec: &Vec<(f64, f64)>) -> LineString {
        tuple_vec.iter().map(|tup| Coord::from(*tup)).collect()
    }

    #[rstest]
    #[case((0.0, 0.0), (1.0, 0.0), 0.0)]
    #[case((0.0, 0.0), (-1.0, 0.0), 0.0)]
    #[case((0.0, 0.0), (0.0, 1.0), FRAC_PI_2)]
    #[case((0.0, 0.0), (0.0, -1.0), FRAC_PI_2)]
    #[case((0.0, 0.0), (1.0, 1.0), FRAC_PI_4)]
    #[case((0.0, 0.0), (-1.0, -1.0), FRAC_PI_4)]
    #[case((0.0, 0.0), (1.0, -1.0), -FRAC_PI_4)]
    fn test_get_normalized_line_azimuth(
        #[case] line_start: (f64, f64),
        #[case] line_end: (f64, f64),
        #[case] expected_aximuth: f64,
    ) {
        let line = geo::Line::new(geo::Coord::from(line_start), geo::Coord::from(line_end));
        let azimuth = get_normalized_line_azimuth(&line);
        assert_abs_diff_eq!(expected_aximuth, azimuth);
    }

    #[rstest]
    #[case(vec![(0.0, 0.0), (10.0, 0.0)], 5.0, vec![(0.0, 0.0), (5.0, 0.0), (10.0, 0.0)])] // Split exactly in two.
    #[case(vec![(0.0, 0.0), (9.0, 0.0)], 4.0, vec![(0.0, 0.0), (4.5, 0.0), (9.0, 0.0)])] // Split exactly in two, float.
    #[case(vec![(0.0, 0.0), (12.0, 0.0)], 5.0, vec![(0.0, 0.0), (6.0, 0.0), (12.0, 0.0)])] // Split in two with leeway.
    #[case(vec![(0.0, 0.0), (9.0, 0.0)], 3.0, vec![(0.0, 0.0), (3.0, 0.0), (6.0, 0.0), (9.0, 0.0)])] // Split exactly in three.
    #[case(vec![(0.0, 0.0), (10.0, 0.0)], 10.0, vec![(0.0, 0.0), (10.0, 0.0)])] // Split by length.
    #[case(vec![(0.0, 0.0), (10.0, 0.0)], 11.0, vec![(0.0, 0.0), (10.0, 0.0)])] // Split by more than length.
    #[case(vec![(0.0, 0.0), (10.0, 0.0)], 0.0, vec![])] // Split by zero.
    #[case(vec![(0.0, 0.0), (10.0, 0.0)], -1.0, vec![])] // Split by negative.
    #[case(vec![(0.0, 0.0), (5.0, 0.0), (9.0, 0.0)], 3.0, vec![(0.0, 0.0), (3.0, 0.0), (6.0, 0.0), (9.0, 0.0)])] // Split linestr with multiple vertices.
    #[case(vec![(0.0, 0.0), (4.5, 0.0), (4.5, 4.5)], 3.0, vec![(0.0, 0.0), (3.0, 0.0), (4.5, 1.5), (4.5, 4.5)])] // Split curving linestr with multiple vertices.
    fn test_sample_points_on_line(
        #[case] input_linestr: Vec<(f64, f64)>,
        #[case] resampling_distance: f64,
        #[case] expected_coordinates: Vec<(f64, f64)>,
    ) {
        let input_linestr = tuple_vec_to_linestring(&input_linestr);
        let result = super::sample_points_on_line(&input_linestr, resampling_distance);

        let expected_coords_linestr: geo::LineString =
            tuple_vec_to_linestring(&expected_coordinates);
        let actual_coords_linestr: geo::LineString =
            result.iter().map(|point| point.coord).collect();
        assert_abs_diff_eq!(
            expected_coords_linestr,
            actual_coords_linestr,
            epsilon = 1e-6
        );
    }
}
