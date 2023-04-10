use std::iter::zip;

use crate::crs::crs_utils::{epsg_4326, epsg_code_to_authority_string, query_utm_crs_info};

use anyhow::anyhow;
use proj::Transform;

use super::primitives::{GeoGraph, NodeIdx};

type NodeIndexerPoint = rstar::primitives::GeomWithData<[f64; 2], NodeIdx>;

pub struct NodeIndexer {
    rtree: rstar::RTree<NodeIndexerPoint>,
    current_index: NodeIdx,
}

impl NodeIndexer {
    pub fn new() -> Self {
        Self {
            rtree: rstar::RTree::new(),
            current_index: 0,
        }
    }

    pub fn get_index_for_coordinate(&mut self, coord: &geo::Coord) -> NodeIdx {
        let coord = [coord.x, coord.y];
        if let Some(point) = self.rtree.locate_at_point(&coord) {
            return point.data;
        }
        self.rtree
            .insert(NodeIndexerPoint::new(coord, self.current_index));
        self.current_index += 1;
        return self.current_index - 1;
    }
}

/// Build a topologically correct GeoGraph from given linestrings. Edge and node data are initialized to defaults.
///
/// Nodes will be created at line endpoints in a topologically correct way, i.e. if two
/// share an endpoint, they will share a common node there.
///
/// Nodes are indexed from zero, in the order of appearance. As an example, this code:
/// ```
/// let my_graph: GeoGraph<(), (), petgraph::Directed> = build_geograph_from_lines(vec![
///     vec![(0.0, 0.0), (1.0, 0.0)].into(),
///     vec![(1.0, 0.0), (2.0, 0.0)].into(),
/// ]);
/// ```
///
/// will create a graph with nodes like this:
/// - idx 0: (0.0, 0.0)
/// - idx 1: (1.0, 0.0)
/// - idx 2: (2.0. 0.0)
///
/// Parameters:
/// - `E`: the data type associeted with edges of the resulting graph.
/// - `N`: the data type associated with nodes of the resulting graph.
/// - `Ty`: the directedness of the resulting graph, e.g. petgraph::Directed.
pub fn build_geograph_from_lines<E: Default, D: Default, Ty: petgraph::EdgeType>(
    lines: Vec<geo::LineString>,
) -> anyhow::Result<GeoGraph<E, D, Ty>> {
    let mut node_indexer = NodeIndexer::new();
    let mut geograph = GeoGraph::new(epsg_4326());
    for (index, line) in lines.into_iter().enumerate() {
        if 2 > line.coords().count() {
            continue;
        }
        let start_point = line.points().nth(0).unwrap();
        let start_node_idx = node_indexer.get_index_for_coordinate(&start_point.into());
        let end_point = line.points().last().unwrap();
        let end_node_idx = node_indexer.get_index_for_coordinate(&end_point.into());
        geograph.insert_edge(start_node_idx, end_node_idx, line)?;
    }

    Ok(geograph)
}

/// Like `build_geograph_from_lines`, with the addition of also initializing the edges with data.
/// The argument `data` should contain the data for each line geometry in matching order. It must have the same
/// length as `lines`.
pub fn build_geograph_from_lines_with_data<E: Default, D: Default, Ty: petgraph::EdgeType>(
    lines: Vec<geo::LineString>,
    data: Vec<E>,
) -> anyhow::Result<GeoGraph<E, D, Ty>> {
    if lines.len() != data.len() {
        return Err(anyhow!(
            "Number of lines ({}) must match number of data ({})",
            lines.len(),
            data.len()
        ));
    }

    let mut node_indexer = NodeIndexer::new();
    let mut geograph = GeoGraph::new(epsg_4326());
    for (line, data_item) in zip(lines.into_iter(), data.into_iter()) {
        if 2 > line.coords().count() {
            continue;
        }
        let start_point = line.points().nth(0).unwrap();
        let start_node_idx = node_indexer.get_index_for_coordinate(&start_point.into());
        let end_point = line.points().last().unwrap();
        let end_node_idx = node_indexer.get_index_for_coordinate(&end_point.into());
        geograph.insert_edge_with_data(start_node_idx, end_node_idx, line, data_item)?;
    }

    Ok(geograph)
}

pub fn get_utm_zone_for_graph<E: Default, N: Default, Ty: petgraph::EdgeType>(
    geograph: &GeoGraph<E, N, Ty>,
) -> anyhow::Result<gdal::spatial_ref::SpatialRef> {
    if !geograph.crs.is_geographic() {
        return Err(anyhow!("The lines are not in a geographic CRS."));
    }
    match geograph.node_map().values().nth(0) {
        Some(node) => {
            let utm_zone_codes =
                query_utm_crs_info(node.geometry.x(), node.geometry.y(), Some("WGS84"))?;
            let utm_zone_code = utm_zone_codes
                .get(0)
                .ok_or_else(|| (anyhow!("No UTM zones found for graph")))?;
            gdal::spatial_ref::SpatialRef::from_epsg(*utm_zone_code)
                .map_err(|err| anyhow!("Could not create SpatialRef from EPSG code. {}", err))
        }
        None => {
            return Err(anyhow!(
                "Could not determine UTM zone for graph because it has no nodes."
            ))
        }
    }
}

/// Project a geograph into the CRS indicated by `to_crs`.
pub fn project_geograph<E: Default, N: Default, Ty: petgraph::EdgeType>(
    geograph: &mut GeoGraph<E, N, Ty>,
    to_crs: &gdal::spatial_ref::SpatialRef,
) -> anyhow::Result<()> {
    let projection = proj::Proj::new_known_crs(
        &epsg_code_to_authority_string(geograph.crs.auth_code()? as u32),
        &epsg_code_to_authority_string(to_crs.auth_code()? as u32),
        None,
    )?;
    for (_, _, par_edges) in geograph.edge_graph_mut().all_edges_mut() {
        for edge in par_edges.iter_mut() {
            edge.geometry.transform(&projection)?;
        }
    }
    for node in geograph.node_map_mut().values_mut() {
        node.geometry.transform(&projection)?;
    }

    geograph.crs = to_crs.clone();
    Ok(())
}

#[cfg(test)]
#[generic_tests::define]
mod tests {

    use std::iter::zip;

    use approx::assert_abs_diff_eq;

    use crate::geograph::{primitives::GeoGraph, utils::build_geograph_from_lines};

    use super::{build_geograph_from_lines_with_data, project_geograph};

    /// Graph type used in tests, holds no extra data for edges or nodes.
    type TestGraph<Ty> = GeoGraph<(), (), Ty>;

    #[test]
    fn test_build_geograph_from_lines<Ty: petgraph::EdgeType>() {
        let node_1_coord = (0.0, 0.0);
        let node_2_coord = (10.0, 0.0);
        let node_3_coord = (20.0, 0.0);
        let node_4_coord = (10.0, 10.0);

        let lines: Vec<geo::LineString> = vec![
            vec![node_1_coord, node_2_coord].into(),
            vec![node_2_coord, node_3_coord].into(),
            vec![node_2_coord, node_4_coord].into(),
        ];
        let graph: TestGraph<Ty> = build_geograph_from_lines(lines.clone()).unwrap();

        // The expected start and end node indices of edges in the graph.
        let expected_edge_indices = [(0, 1), (1, 2), (1, 3)];
        assert_eq!(expected_edge_indices.len(), graph.edge_graph().edge_count());
        for (edge_index, (start_node_index, end_node_index)) in
            expected_edge_indices.iter().enumerate()
        {
            let expected_line = lines.get(edge_index as usize).unwrap();
            let edge = graph
                .edge_graph()
                .edge_weight(*start_node_index, *end_node_index)
                .unwrap()
                .get(0)
                .unwrap();
            assert_eq!(*expected_line, edge.geometry);
        }

        // The expected node coordinates in order of the expected node indices.
        let expected_node_coords = [node_1_coord, node_2_coord, node_3_coord, node_4_coord];
        assert_eq!(graph.node_map().len(), expected_node_coords.len());
        for (node_index, expected_coord) in expected_node_coords.iter().enumerate() {
            let node = graph.node_map().get(&(node_index as u64)).unwrap();
            assert_eq!(*expected_coord, (node.geometry.x(), node.geometry.y()));
        }
    }

    #[test]
    fn test_build_geograph_from_lines_with_data<Ty: petgraph::EdgeType>() {
        let node_1_coord = (0.0, 0.0);
        let node_2_coord = (10.0, 0.0);
        let node_3_coord = (20.0, 0.0);

        let lines: Vec<geo::LineString> = vec![
            vec![node_1_coord, node_2_coord].into(),
            vec![node_2_coord, node_3_coord].into(),
        ];
        let data = vec!["a".to_string(), "b".to_string()];

        let graph: GeoGraph<String, String, Ty> =
            build_geograph_from_lines_with_data(lines, data.clone()).unwrap();

        let expected_edge_indices = [(0, 1), (1, 2)];

        for (expected_data_item, (start_node_id, end_node_id)) in zip(data, expected_edge_indices) {
            let edge = graph
                .edge_graph()
                .edge_weight(start_node_id, end_node_id)
                .unwrap();
            // We expect one edge, no parallel edges.
            assert_eq!(1, edge.len());
            assert_eq!(*expected_data_item, edge.get(0).unwrap().data);
        }
    }

    #[test]
    fn test_project_geograph<Ty: petgraph::EdgeType>() {
        // EPSG 4326 coordinates.
        let node_1_coord = (139.7895073, 35.6862101);
        let node_2_coord = (139.7912979, 35.6870132);
        let node_3_coord = (139.7919128, 35.6862357);

        let lines: Vec<geo::LineString> = vec![
            vec![node_1_coord, node_2_coord].into(),
            vec![node_2_coord, node_3_coord].into(),
        ];

        let mut graph: TestGraph<Ty> = build_geograph_from_lines(lines).unwrap();
        graph.crs = crate::crs::crs_utils::epsg_4326();

        let target_crs = gdal::spatial_ref::SpatialRef::from_epsg(32654).unwrap(); // UTM zone 54N
        project_geograph(&mut graph, &target_crs).unwrap();

        // Computed using https://coordinates-converter.com/
        let exp_node_1_coord = (390467.986, 3949820.494);
        let exp_node_2_coord = (390631.113, 3949907.576);
        let exp_node_3_coord = (390685.694, 3949820.653);

        let expected_node_coords = [exp_node_1_coord, exp_node_2_coord, exp_node_3_coord];

        // Millimeter tolerance.
        let epsilon = 1e-3;

        for (index, (x, y)) in expected_node_coords.iter().enumerate() {
            let node_geom = graph.node_map().get(&(index as u64)).unwrap().geometry;
            assert_abs_diff_eq!(node_geom, geo::Point::new(*x, *y), epsilon = epsilon);
        }

        let expected_node_indices = [(0, 1), (1, 2)];
        for (start_node_idx, end_node_idx) in expected_node_indices {
            let edge = graph
                .edge_graph()
                .edge_weight(start_node_idx, end_node_idx)
                .unwrap()
                .get(0)
                .unwrap();
            let start_node_geom = edge.geometry.points().nth(0).unwrap();
            let (x, y) = expected_node_coords.get(start_node_idx as usize).unwrap();
            assert_abs_diff_eq!(start_node_geom, geo::Point::new(*x, *y), epsilon = epsilon);
            let end_node_geom = edge.geometry.points().last().unwrap();
            let (x, y) = expected_node_coords.get(end_node_idx as usize).unwrap();
            assert_abs_diff_eq!(end_node_geom, geo::Point::new(*x, *y), epsilon = epsilon);
        }

        assert_eq!(
            graph.crs.auth_code().unwrap(),
            target_crs.auth_code().unwrap()
        );
    }

    #[instantiate_tests(<petgraph::Directed>)]
    mod directed {}

    #[instantiate_tests(<petgraph::Undirected>)]
    mod undirected {}
}
