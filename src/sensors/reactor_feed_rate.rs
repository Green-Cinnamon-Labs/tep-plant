/* tep/sensors/reactor_feed_rate.rs */

/* XMEAS(6), Reactor Feed Rate (Stream 6) — kscmh, publicado por Measured. */
#[monjolo::sensor(key = "xmeas.stream6.flow_rate")]
pub struct ReactorFeedRate;
