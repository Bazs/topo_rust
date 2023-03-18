use crate::crs::crs_utils::{epsg_code_to_authority_string, EpsgCode};

use super::georef_lines::{get_utm_zone_for_lines, project_lines, GeoreferencedLines};

pub fn ensure_gt_proposal_same_projected_crs(
    gt_georef_lines: &mut GeoreferencedLines,
    proposal_georef_lines: &mut GeoreferencedLines,
) -> anyhow::Result<()> {
    if gt_georef_lines.spatial_ref.is_projected() {
        if gt_georef_lines.spatial_ref.auth_code()?
            != proposal_georef_lines.spatial_ref.auth_code()?
        {
            log::info!(
                "Projecting proposal lines to {}",
                epsg_code_to_authority_string(gt_georef_lines.spatial_ref.auth_code()? as EpsgCode)
            );
            *proposal_georef_lines =
                project_lines(&proposal_georef_lines, &gt_georef_lines.spatial_ref)?;
        }
    } else {
        let utm_zone = get_utm_zone_for_lines(&gt_georef_lines)?;

        log::info!(
            "Projecting ground truth and proposal lines to {}",
            epsg_code_to_authority_string(utm_zone.auth_code()? as EpsgCode)
        );

        *gt_georef_lines = project_lines(gt_georef_lines, &utm_zone)?;
        *proposal_georef_lines = project_lines(&proposal_georef_lines, &utm_zone)?;
    }
    Ok(())
}
