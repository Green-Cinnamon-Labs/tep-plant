/* Documentação: docs/19-atuadores.md */

#[monjolo::actuator(key = "valve.stripper_steam.position", config = "state.valves.stripper_steam_valve")]
pub struct StripperSteam {
    #[command]
    command: f64,
    #[state]
    position: f64,
}

impl StripperSteam {
    fn dynamics(&self) -> f64 {
        let tau = 120.0 / 3600.0;
        (self.command() - self.position()) / tau
    }
}
