/* tep/actuators/stripper_steam.rs */

/* XMV-9: Stripper Steam Valve. VTAU(9) = 120s — a mais lenta das 12 (não a de resfriamento do
condensador, como docs/_deprecated_1.rs sugere).
*/
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
