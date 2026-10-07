/* tep/sensors/purge_component_c.rs */

/* XMEAS(31), Purge Gas Analysis — Component C (mol%), publicado por PurgeAnalyzer. */
#[monjolo::sensor(key = "xmeas.stream9.component.c", noise = 0.25)]
pub struct PurgeComponentC;
