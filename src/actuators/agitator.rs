/* Documentação: docs/19-atuadores.md */

#[monjolo::actuator(key = "agitator.speed", config = "state.valves.agitator_speed")]
pub struct Agitator {
    #[command]
    command: f64,
    #[state]
    speed: f64,
}

impl Agitator {
    fn dynamics(&self) -> f64 {
        let tau = 5.0 / 3600.0;
        (self.command() - self.speed()) / tau
    }
}
