/* tep/sensors/purge_component_d.rs */

/* XMEAS(32), Purge Gas Analysis — Component D (mol%), publicado por PurgeAnalyzer. */
#[monjolo::sensor(key = "xmeas.stream9.component.d", noise = 0.1)]
pub struct PurgeComponentD;
