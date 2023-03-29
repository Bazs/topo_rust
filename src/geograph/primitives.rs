use std::collections::HashMap;

use anyhow::anyhow;

/// Edge of a geospatial graph.
/// Parameters:
/// - `D`: type of associated data.
pub struct GeoEdge<D: Default> {
    pub geometry: geo::LineString,
    pub data: D,
}

impl<D: Default> GeoEdge<D> {
    /// Create new edge with given geometry and default data.
    pub fn new(geometry: geo::LineString) -> Self {
        Self {
            geometry,
            data: D::default(),
        }
    }

    /// Create new edge with given geometry and data.
    pub fn new_with_data(geometry: geo::LineString, data: D) -> Self {
        Self { geometry, data }
    }
}

/// Index type used for nodes of a geospatial graph.
pub type NodeIdx = u64;

/// Node of a geospatial graph.
/// /// Parameters:
/// - `D`: type of associated data.
pub struct GeoNode<D: Default> {
    pub geometry: geo::Point,
    pub data: D,
}

impl<D: Default> GeoNode<D> {
    /// Create new node with given geometry and default data.
    pub fn new(geometry: geo::Point) -> Self {
        Self {
            geometry,
            data: D::default(),
        }
    }

    /// Create new node with given geometry and data.
    pub fn new_with_data(geometry: geo::Point, data: D) -> Self {
        Self { geometry, data }
    }
}

/// Graph of geospatial edges. Parallel edges are supported because the edge weight is a vector of GeoEdge.
/// Parameters:
/// - `E`: the data type associated with edges.
/// - `Ty`: whether the graph is directed or undirected, see petgraph documentation for details.
pub type EdgeGraph<E, Ty> = petgraph::graphmap::GraphMap<NodeIdx, Vec<GeoEdge<E>>, Ty>;

/// Map containing data associated with the nodes of a geospatial graph, indexed by node index.
/// Parameters:
/// - `N`: the data type associated with nodes.
pub type NodeMap<N> = HashMap<NodeIdx, GeoNode<N>>;

/// Geospatial graph. Edges are stored in a map-based graph, which is indexed by start and end node indices.
/// Data associated with nodes is stored in a map. The `crs` member defines the coordinate reference system.
///
/// Parameters:
/// - `E`: the data type associated with edges.
/// - `N`: the data type associated with nodes.
/// - `Ty`: whether the graph is directed or undirected, see petgraph documentation for details.
pub struct GeoGraph<E: Default, N: Default, Ty: petgraph::EdgeType> {
    edge_graph: EdgeGraph<E, Ty>,
    node_map: NodeMap<N>,
    pub crs: gdal::spatial_ref::SpatialRef,
}

impl<E: Default, N: Default, Ty: petgraph::EdgeType> GeoGraph<E, N, Ty> {
    /// Create an empty graph.
    pub fn new(crs: gdal::spatial_ref::SpatialRef) -> Self {
        Self {
            edge_graph: EdgeGraph::new(),
            node_map: HashMap::new(),
            crs: crs,
        }
    }

    pub fn edge_graph(&self) -> &EdgeGraph<E, Ty> {
        &self.edge_graph
    }

    pub fn node_map(&self) -> &NodeMap<N> {
        &self.node_map
    }

    pub fn insert_edge(
        &mut self,
        start_node_idx: NodeIdx,
        end_node_idx: NodeIdx,
        geometry: geo::LineString,
    ) -> anyhow::Result<()> {
        self.insert_edge_with_data(start_node_idx, end_node_idx, geometry, E::default())
    }

    pub fn insert_edge_with_data(
        &mut self,
        start_node_idx: NodeIdx,
        end_node_idx: NodeIdx,
        geometry: geo::LineString,
        data: E,
    ) -> anyhow::Result<()> {
        if 2 > geometry.coords().count() {
            return Err(anyhow!("Cannot insert edge with less than two points"));
        }

        let line_start_point = geometry.coords().nth(0).unwrap();
        let line_end_point = geometry.coords().last().unwrap();

        self.insert_node(start_node_idx, (*line_start_point).into())?;
        self.insert_node(end_node_idx, (*line_end_point).into())?;

        if let Some(edge_vec) = self
            .edge_graph
            .edge_weight_mut(start_node_idx, end_node_idx)
        {
            // TODO consider having a "parallel edge idx" in the function signature and check if that parallel edge idx exsits already.
            edge_vec.push(GeoEdge::new_with_data(geometry, data))
        } else {
            self.edge_graph.add_edge(
                start_node_idx,
                end_node_idx,
                vec![GeoEdge::new_with_data(geometry, data)],
            );
        }

        Ok(())
    }

    pub fn insert_node(&mut self, idx: NodeIdx, geometry: geo::Point) -> anyhow::Result<()> {
        if let Some(node) = self.node_map.get(&idx) {
            if node.geometry != geometry {
                return Err(anyhow!(
                    "Node with the same index ({}) but different geometry already exists",
                    idx
                ));
            }
        } else {
            self.node_map.insert(idx, GeoNode::new(geometry));
        }
        Ok(())
    }
}

pub type UnGeoGraph<E, N> = GeoGraph<E, N, petgraph::Undirected>;
pub type DiGeoGraph<E, N> = GeoGraph<E, N, petgraph::Directed>;
