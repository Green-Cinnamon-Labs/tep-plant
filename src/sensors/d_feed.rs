/* tep/sensors/d_feed.rs */

/* XMEAS(2), D Feed (Stream 2) — kg/hr, publicado por Measured. */
#[monjolo::sensor(key = "xmeas.stream2.flow_rate", noise = 18.0)]
pub struct DFeed;
