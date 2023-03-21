use super::primitives::{GeoGraph, NodeIdx};

type NodeIndexerPoint = rstar::primitives::GeomWithData<[f64; 2], NodeIdx>;

struct NodeIndexer {
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

/// Build a topologically correct GeoGraph from given linestrings. Edge and node data is initialized to defaults.
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
    let mut geograph = GeoGraph::new();
    for line in lines.into_iter() {
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

#[cfg(test)]
#[generic_tests::define]
mod tests {

    use crate::geograph::{primitives::GeoGraph, utils::build_geograph_from_lines};

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

    #[instantiate_tests(<petgraph::Directed>)]
    mod directed {}

    #[instantiate_tests(<petgraph::Undirected>)]
    mod undirected {}
}
