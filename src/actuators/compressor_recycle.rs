/* tep/actuators/compressor_recycle.rs */

/* XMV-5: Compressor Recycle Valve. VTAU(5) = 7s. */
#[monjolo::actuator(key = "valve.compressor_recycle.position", config = "state.valves.compressor_recycle_valve")]
pub struct CompressorRecycle {
    #[command]
    command: f64,
    #[state]
    position: f64,
}

impl CompressorRecycle {
    fn dynamics(&self) -> f64 {
        let tau = 7.0 / 3600.0;
        (self.command() - self.position()) / tau
    }
}
