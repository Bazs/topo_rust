pub struct TopoResult {
    precision: f64,
    recall: f64,
    f1_score: f64,
}

pub fn calculate_topo(
    proposal: &Vec<geo::LineString>,
    ground_truth: &Vec<geo::LineString>,
) -> anyhow::Result<TopoResult> {
    unimplemented!();
}
