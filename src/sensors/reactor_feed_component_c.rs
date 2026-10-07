/* tep/sensors/reactor_feed_component_c.rs */

/* XMEAS(25), Reactor Feed Analysis — Component C (mol%), publicado por ReactorFeedAnalyzer. */
#[monjolo::sensor(key = "xmeas.stream6.component.c", noise = 0.25)]
pub struct ReactorFeedComponentC;
