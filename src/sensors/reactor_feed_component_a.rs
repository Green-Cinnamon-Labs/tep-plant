/* tep/sensors/reactor_feed_component_a.rs */

/* XMEAS(23), Reactor Feed Analysis — Component A (mol%), publicado por ReactorFeedAnalyzer. */
#[monjolo::sensor(key = "xmeas.stream6.component.a", noise = 0.25)]
pub struct ReactorFeedComponentA;
