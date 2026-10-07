/* tep/sensors/reactor_feed_component_e.rs */

/* XMEAS(27), Reactor Feed Analysis — Component E (mol%), publicado por ReactorFeedAnalyzer. */
#[monjolo::sensor(key = "xmeas.stream6.component.e", noise = 0.25)]
pub struct ReactorFeedComponentE;
