/* tep/sensors/reactor_feed_component_b.rs */

/* XMEAS(24), Reactor Feed Analysis — Component B (mol%), publicado por ReactorFeedAnalyzer. */
#[monjolo::sensor(key = "xmeas.stream6.component.b", noise = 0.1)]
pub struct ReactorFeedComponentB;
