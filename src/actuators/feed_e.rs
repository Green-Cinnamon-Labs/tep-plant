/* Documentação: docs/19-atuadores.md */

#[monjolo::actuator(key = "valve.feed_e.position", config = "state.valves.e_feed")]
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
