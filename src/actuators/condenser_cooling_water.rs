/* tep/actuators/condenser_cooling_water.rs */

/* XMV-11: Condenser Cooling Water Flow. VTAU(11) = 5s — igual à maioria das demais (não 120s, como
docs/_deprecated_1.rs sugere).
*/
#[monjolo::actuator(key = "valve.condenser_cooling_water.position")]
pub struct CondenserCoolingWater {
    #[command]
    command: f64,
    #[state]
    position: f64,
}

impl CondenserCoolingWater {
    fn dynamics(&self) -> f64 {
        let tau = 5.0 / 3600.0;
        (self.command() - self.position()) / tau
    }
}
