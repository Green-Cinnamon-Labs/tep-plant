/* tep/sensors/e_feed.rs */

/* XMEAS(3), E Feed (Stream 3) — kg/hr, publicado por Measured. */
#[monjolo::sensor(key = "xmeas.stream3.flow_rate")]
pub struct EFeed;
