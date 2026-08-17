/* tep/actuators/feed_e.rs */

/* XMV-2: E Feed Flow. VTAU(2) = 8s. */
#[monjolo::actuator(key = "valve.feed_e.position")]
pub struct FeedE {
    #[command]
    command: f64,
    #[state]
    position: f64,
}

impl FeedE {
    fn dynamics(&self) -> f64 {
        let tau = 8.0 / 3600.0;
        (self.command() - self.position()) / tau
    }
}
