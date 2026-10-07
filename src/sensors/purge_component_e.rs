/* tep/sensors/purge_component_e.rs */

/* XMEAS(33), Purge Gas Analysis — Component E (mol%), publicado por PurgeAnalyzer. */
#[monjolo::sensor(key = "xmeas.stream9.component.e", noise = 0.25)]
pub struct PurgeComponentE;
