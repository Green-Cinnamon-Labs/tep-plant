/* tep/sensors/purge_component_f.rs */

/* XMEAS(34), Purge Gas Analysis — Component F (mol%), publicado por PurgeAnalyzer. */
#[monjolo::sensor(key = "xmeas.stream9.component.f", noise = 0.025)]
pub struct PurgeComponentF;
