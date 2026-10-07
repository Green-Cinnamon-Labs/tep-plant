/* tep/sensors/purge_component_b.rs */

/* XMEAS(30), Purge Gas Analysis — Component B (mol%), publicado por PurgeAnalyzer. */
#[monjolo::sensor(key = "xmeas.stream9.component.b", noise = 0.1)]
pub struct PurgeComponentB;
