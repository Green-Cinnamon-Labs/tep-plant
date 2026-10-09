/* Documentação: docs/19-atuadores.md */

#[monjolo::actuator(key = "valve.separator_underflow.position", config = "state.valves.separator_underflow")]
pub struct SeparatorUnderflow {
    #[command]
    command: f64,
    #[state]
    position: f64,
}

impl SeparatorUnderflow {
    fn dynamics(&self) -> f64 {
        let tau = 5.0 / 3600.0;
        (self.command() - self.position()) / tau
    }
}
