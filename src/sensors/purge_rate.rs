/* tep/sensors/purge_rate.rs */

/* XMEAS(10), Purge Rate (Stream 9) — kscmh, publicado por Measured. */
#[monjolo::sensor(key = "xmeas.stream9.flow_rate", noise = 0.0017)]
pub struct PurgeRate;
