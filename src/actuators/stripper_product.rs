/* tep/actuators/stripper_product.rs */

/* XMV-8: Stripper Liquid Product Flow. VTAU(8) = 5s. */
#[monjolo::actuator(key = "valve.stripper_product.position", config = "state.valves.stripper_product")]
pub struct StripperProduct {
    #[command]
    command: f64,
    #[state]
    position: f64,
}

impl StripperProduct {
    fn dynamics(&self) -> f64 {
        let tau = 5.0 / 3600.0;
        (self.command() - self.position()) / tau
    }
}
