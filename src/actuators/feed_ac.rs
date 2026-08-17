/* tep/actuators/feed_ac.rs */

/* XMV-4: A&C Feed Flow (alimentação combinada). VTAU(4) = 9s. */
#[monjolo::actuator(key = "valve.feed_ac.position", config = "state.valves.a_c_feed")]
pub struct FeedAc {
    #[command]
    command: f64,
    #[state]
    position: f64,
}

impl FeedAc {
    fn dynamics(&self) -> f64 {
        let tau = 9.0 / 3600.0;
        (self.command() - self.position()) / tau
    }
}
