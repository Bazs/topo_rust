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
    if resampling_distance <= 0.0 {
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
        while (next_original_vert_dist - prev_inserted_dist) > resampling_distance {
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

#[cfg(test)]
mod tests {
    extern crate approx;
    use approx::assert_abs_diff_eq;
    use geo::{Coord, LineString};
    use rstest::rstest;

    fn tuple_vec_to_linestring(tuple_vec: &Vec<(f64, f64)>) -> LineString {
        tuple_vec.iter().map(|tup| Coord::from(*tup)).collect()
    }

    #[rstest]
    #[case(vec![(0.0, 0.0), (10.0, 0.0)], 5.0, Some(vec![(0.0, 0.0), (5.0, 0.0), (10.0, 0.0)]))] // Split exactly in two.
    #[case(vec![(0.0, 0.0), (9.0, 0.0)], 4.0, Some(vec![(0.0, 0.0), (4.5, 0.0), (9.0, 0.0)]))] // Split exactly in two, float.
    #[case(vec![(0.0, 0.0), (12.0, 0.0)], 5.0, Some(vec![(0.0, 0.0), (6.0, 0.0), (12.0, 0.0)]))] // Split in two with leeway.
    #[case(vec![(0.0, 0.0), (9.0, 0.0)], 3.0, Some(vec![(0.0, 0.0), (3.0, 0.0), (6.0, 0.0), (9.0, 0.0)]))] // Split exactly in three.
    #[case(vec![(0.0, 0.0), (10.0, 0.0)], 10.0, Some(vec![(0.0, 0.0), (10.0, 0.0)]))] // Split by length.
    #[case(vec![(0.0, 0.0), (10.0, 0.0)], 11.0, Some(vec![(0.0, 0.0), (10.0, 0.0)]))] // Split by more than length.
    #[case(vec![(0.0, 0.0), (10.0, 0.0)], 0.0, None)] // Split by zero.
    #[case(vec![(0.0, 0.0), (10.0, 0.0)], -1.0, None)] // Split by negative.
    #[case(vec![(0.0, 0.0), (5.0, 0.0), (9.0, 0.0)], 3.0, Some(vec![(0.0, 0.0), (3.0, 0.0), (6.0, 0.0), (9.0, 0.0)]))] // Split linestr with multiple vertices.
    #[case(vec![(0.0, 0.0), (4.5, 0.0), (4.5, 4.5)], 3.0, Some(vec![(0.0, 0.0), (3.0, 0.0), (4.5, 1.5), (4.5, 4.5)]))] // Split curving linestr with multiple vertices.
    fn test_resample_line(
        #[case] input_linestr: Vec<(f64, f64)>,
        #[case] resampling_distance: f64,
        #[case] expected_linestr: Option<Vec<(f64, f64)>>,
    ) {
        let input_linestr = tuple_vec_to_linestring(&input_linestr);
        let result = super::resample_line(&input_linestr, resampling_distance);

        match expected_linestr {
            Some(expected_linestr) => {
                let expected_linestr = tuple_vec_to_linestring(&expected_linestr);
                assert_abs_diff_eq!(expected_linestr, result.unwrap(), epsilon = 1e-6);
            }
            None => {
                assert!(result.is_none())
            }
        }
    }
}
