/* tep/actuators/feed_a.rs */

/* XMV-3: A Feed Flow. VTAU(3) = 6s. */
#[monjolo::actuator(key = "valve.feed_a.position", config = "state.valves.a_feed")]
pub struct FeedA {
    #[command]
    command: f64,
    #[state]
    position: f64,
}

impl FeedA {
    fn dynamics(&self) -> f64 {
        let tau = 6.0 / 3600.0;
        (self.command() - self.position()) / tau
    }
}
