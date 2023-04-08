use crate::{
    crs::crs_utils::{epsg_code_to_authority_string, EpsgCode},
    geograph::{
        primitives::GeoGraph,
        utils::{get_utm_zone_for_graph, project_geograph},
    },
};

pub fn ensure_gt_proposal_in_same_projected_crs<
    E: Default,
    N: Default,
    Ty: petgraph::EdgeType,
>(
    gt_graph: &mut GeoGraph<E, N, Ty>,
    proposal_graph: &mut GeoGraph<E, N, Ty>,
) -> anyhow::Result<()> {
    if gt_graph.crs.is_projected() {
        if gt_graph.crs.auth_code()? != proposal_graph.crs.auth_code()? {
            log::info!(
                "Projecting proposal graph to {}",
                epsg_code_to_authority_string(gt_graph.crs.auth_code()? as EpsgCode)
            );
            project_geograph(proposal_graph, &gt_graph.crs)?;
        }
    } else {
        let utm_zone = get_utm_zone_for_graph(&gt_graph)?;

        log::info!(
            "Projecting ground truth and proposal lines to {}",
            epsg_code_to_authority_string(utm_zone.auth_code()? as EpsgCode)
        );

        project_geograph(gt_graph, &utm_zone)?;
        project_geograph(proposal_graph, &utm_zone)?;
    }
    Ok(())
}
