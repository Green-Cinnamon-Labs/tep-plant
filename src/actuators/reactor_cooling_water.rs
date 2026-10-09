/* Documentação: docs/19-atuadores.md */

#[monjolo::actuator(key = "valve.reactor_cooling_water.position", config = "state.valves.reactor_cooling_water")]
pub struct ReactorCoolingWater {
    #[command]
    command: f64,
    #[state]
    position: f64,
}

impl ReactorCoolingWater {
    fn dynamics(&self) -> f64 {
        let tau = 5.0 / 3600.0;
        (self.command() - self.position()) / tau
    }
}
