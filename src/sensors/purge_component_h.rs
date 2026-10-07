/* tep/sensors/purge_component_h.rs */

/* XMEAS(36), Purge Gas Analysis — Component H (mol%), publicado por PurgeAnalyzer. */
#[monjolo::sensor(key = "xmeas.stream9.component.h", noise = 0.05)]
pub struct PurgeComponentH;
