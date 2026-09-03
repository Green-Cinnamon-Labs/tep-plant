/* tep/sensors/ac_feed.rs */

/* XMEAS(4), A and C Feed (Stream 4) — kscmh, publicado por Measured. */
#[monjolo::sensor(key = "xmeas.stream4.flow_rate")]
pub struct AcFeed;
