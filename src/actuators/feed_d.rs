/* Documentação: docs/19-atuadores.md */

#[monjolo::actuator(key = "valve.feed_d.position", config = "state.valves.d_feed")]
pub struct FeedD {
    #[command]
    command: f64,
    #[state]
    position: f64,
}

impl FeedD {
    fn dynamics(&self) -> f64 {
        let tau = 8.0 / 3600.0;
        (self.command() - self.position()) / tau
    }
}
