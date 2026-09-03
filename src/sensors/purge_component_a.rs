/* tep/sensors/purge_component_a.rs */

/* XMEAS(29), Purge Gas Analysis — Component A (mol%), publicado por PurgeAnalyzer. */
#[monjolo::sensor(key = "xmeas.stream9.component.a")]
pub struct PurgeComponentA;
