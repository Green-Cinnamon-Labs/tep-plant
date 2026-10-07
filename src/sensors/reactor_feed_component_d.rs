/* tep/sensors/reactor_feed_component_d.rs */

/* XMEAS(26), Reactor Feed Analysis — Component D (mol%), publicado por ReactorFeedAnalyzer. */
#[monjolo::sensor(key = "xmeas.stream6.component.d", noise = 0.1)]
pub struct ReactorFeedComponentD;
