/* tep/actuators/agitator.rs */

/* XMV-12: Agitator Speed. VTAU(12) = 5s. Único na planta, sem chave por nome. */
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
