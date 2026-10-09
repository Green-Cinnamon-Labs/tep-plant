/* Documentação: docs/19-atuadores.md */

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
