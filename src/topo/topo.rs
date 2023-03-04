use std::{
    collections::{HashMap, HashSet},
    f64::consts::FRAC_PI_2,
};

use anyhow::anyhow;
use geo::{CoordsIter, EuclideanLength};
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use kdtree::distance::squared_euclidean;
use rayon::prelude::*;

use crate::geofile::feature::Feature;

#[derive(PartialEq, Debug)]
pub struct F1ScoreResult {
    precision: f64,
    recall: f64,
    f1_score: f64,
}

pub struct TopoResult {
    pub f1_score_result: F1ScoreResult,
    pub ground_truth_nodes: Vec<TopoNode>,
    pub proposal_nodes: Vec<TopoNode>,
}

pub struct TopoParams {
    pub resampling_distance: f64,
    pub hole_radius: f64,
}

pub fn calculate_topo<'a>(
    proposal: &Vec<geo::LineString>,
    ground_truth: &Vec<geo::LineString>,
    params: &TopoParams,
) -> anyhow::Result<TopoResult> {
    // Interpolate the edges.
    log::info!("Sampling points on proposal lines");
    let proposal_points = sample_points_on_lines(proposal, params.resampling_distance);
    let mut proposal_nodes = road_points_to_topo_nodes(proposal_points);
    log::info!("Sampling points on ground truth lines");
    let ground_truth_points: Vec<RoadPoint> =
        sample_points_on_lines(ground_truth, params.resampling_distance);
    let mut ground_truth_nodes = road_points_to_topo_nodes(ground_truth_points);
    log::info!("Building ground truth point lookup tree");
    let ground_truth_kdtree = build_kdtree_from_nodes(&ground_truth_nodes)?;

    log::info!(
        "Matching {} proposal points to {} ground truth points",
        proposal_nodes.len(),
        ground_truth_nodes.len()
    );
    // Get the squared distances and indices of the GT nodes within range, if there are any within hole radius.
    let squared_hole_radius = params.hole_radius.powi(2);
    let progress_style = ProgressStyle::with_template(
        "{wide_bar} {pos}/{len} {percent}% elapsed: {elapsed_precise}",
    )
    .unwrap();
    log::info!("Looking up ground truth nodes within hole radius");
    let prop_node_and_gt_nodes_result: Result<Vec<_>, anyhow::Error> = proposal_nodes
        .par_iter_mut()
        .progress_with_style(progress_style)
        .map(|proposal_node| {
            let gt_distances_and_indices = ground_truth_kdtree
                .within(
                    &<[f64; 2]>::from(proposal_node.road_point.coord),
                    squared_hole_radius,
                    &squared_euclidean,
                )
                .or_else(|error| Err(anyhow!("Could not get nearest GT node, {}", error)))?;
            Ok((proposal_node, gt_distances_and_indices))
        })
        .collect();
    let mut matched_gt_distance_and_idx = prop_node_and_gt_nodes_result?;

    log::info!("Determining matches for proposal nodes");
    let mut matched_gt_ids = HashSet::new();
    let progress_bar = ProgressBar::new(matched_gt_distance_and_idx.len() as u64);
    for (proposal_node, gt_distances_and_indices) in matched_gt_distance_and_idx.iter_mut() {
        for (squared_distance, gt_idx) in gt_distances_and_indices {
            if !matched_gt_ids.contains(gt_idx) {
                let match_distance = squared_distance.sqrt();

                proposal_node.matched = true;
                proposal_node.match_distance = Some(match_distance);

                let mut gt_node = ground_truth_nodes
                    .get_mut(**gt_idx as usize)
                    .ok_or_else(|| anyhow!("No such GT node"))?;
                gt_node.matched = true;
                gt_node.match_distance = Some(match_distance);
                
                matched_gt_ids.insert(gt_idx);
                break;
            }
        }
        progress_bar.inc(1);
    }

    let true_positive_count = matched_gt_ids.len();
    let false_positive_count = proposal_nodes.len() - true_positive_count;
    let false_negative_count = ground_truth_nodes.len() - true_positive_count;
    let precision =
        true_positive_count as f64 / (true_positive_count + false_positive_count) as f64;
    let recall = true_positive_count as f64 / (true_positive_count + false_negative_count) as f64;
    let f1_score = 2.0 * precision * recall / (precision + recall);
    Ok(TopoResult {
        f1_score_result: F1ScoreResult {
            precision,
            recall,
            f1_score,
        },
        ground_truth_nodes,
        proposal_nodes,
    })
}

struct RoadPoint {
    coord: geo::Coord,
    azimuth: f64,
}

pub struct TopoNode {
    road_point: RoadPoint,
    id: i64,
    matched: bool,
    match_distance: Option<f64>,
}

impl From<&TopoNode> for Feature {
    fn from(node: &TopoNode) -> Self {
        Self {
            geometry: geo::Geometry::Point(geo::Point::from(node.road_point.coord)),
            attributes: Some(HashMap::from([
                ("id".to_string(), node.id.to_string()),
                ("matched".to_string(), node.matched.to_string()),
                (
                    "match_distance".to_string(),
                    match node.match_distance {
                        Some(distance) => distance.to_string(),
                        None => "".to_string(),
                    },
                ),
            ])),
        }
    }
}

impl TopoNode {
    fn new(point: RoadPoint, id: i64) -> Self {
        TopoNode {
            road_point: point,
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

fn road_points_to_topo_nodes(road_points: Vec<RoadPoint>) -> Vec<TopoNode> {
    road_points
        .into_iter()
        .enumerate()
        .map(|(idx, road_point)| TopoNode::new(road_point, idx as i64))
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
        azimuth: get_normalized_line_azimuth(&linestr.lines().last().unwrap()), // TODO create the line in a different way, iterating through the lines() is very wasteful
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
    use rstest::{fixture, rstest};
    use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

    use super::{
        calculate_topo, get_normalized_line_azimuth, sample_points_on_line, F1ScoreResult,
        TopoParams,
    };

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
        let input_linestr: geo::LineString = input_linestr.into();
        let result = sample_points_on_line(&input_linestr, resampling_distance);

        let expected_coords_linestr: geo::LineString = expected_coordinates.into();
        let actual_coords_linestr: geo::LineString =
            result.iter().map(|point| point.coord).collect();
        assert_abs_diff_eq!(
            expected_coords_linestr,
            actual_coords_linestr,
            epsilon = 1e-6
        );
    }

    #[fixture]
    fn default_topo_params() -> TopoParams {
        TopoParams {
            hole_radius: 6.0,
            resampling_distance: 11.0,
        }
    }

    #[rstest]
    #[case(vec![(0.0, 0.0), (5.0, 0.0), (11.0, 0.0)], vec![(0.0, 0.0), (5.0, 0.0), (11.0, 0.0)], F1ScoreResult {
        f1_score: 1.0,
        precision: 1.0,
        recall: 1.0
    })] // Perfectly matching lines.
    #[case(vec![(0.0, 0.0), (5.0, 0.0), (11.0, 0.0)], vec![(0.0, 0.0), (5.0, 0.0), (20.0, 0.0)], F1ScoreResult {
        f1_score: 0.5,
        precision: 0.5,
        recall: 0.5
    })] // One line endpoint out of match range.
    fn test_calculate_topo_two_lines(
        #[case] proposal_line_coords: Vec<(f64, f64)>,
        #[case] ground_truth_line_coods: Vec<(f64, f64)>,
        #[case] expected_result: F1ScoreResult,
        default_topo_params: TopoParams,
    ) {
        let proposal_line: geo::LineString = proposal_line_coords.into();
        let ground_truth_line: geo::LineString = ground_truth_line_coods.into();

        let result = calculate_topo(
            &vec![proposal_line],
            &vec![ground_truth_line],
            &default_topo_params,
        );
        assert!(result.is_ok());
        assert_eq!(expected_result, result.unwrap().f1_score_result)
    }
}
