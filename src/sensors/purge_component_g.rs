/* tep/sensors/purge_component_g.rs */

/* XMEAS(35), Purge Gas Analysis — Component G (mol%), publicado por PurgeAnalyzer. */
#[monjolo::sensor(key = "xmeas.stream9.component.g", noise = 0.05)]
pub struct PurgeComponentG;
