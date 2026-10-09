/* Documentação: docs/20-diagnosticos.md */

#[monjolo::dynamic_model]
pub struct ShutdownDetector {
    #[need(key = "xmeas.reactor.pressure")]
    reactor_pressure_kpa: f64,
    #[need(key = "reactor.liquid_volume")]
    reactor_liquid_volume: f64,
    #[need(key = "reactor.temperature")]
    reactor_temperature: f64,
    #[need(key = "separator.liquid_volume")]
    separator_liquid_volume: f64,
    #[need(key = "stripper.liquid_volume")]
    stripper_liquid_volume: f64,

    #[offer(key = "status.shutdown_detected")]
    shutdown_detected: f64,
}

impl ShutdownDetector {
    fn compute(&self) {
        let shutdown_detected = self.reactor_pressure_kpa() > 3000.0
            || self.reactor_liquid_volume() / 35.3145 > 24.0
            || self.reactor_liquid_volume() / 35.3145 < 2.0
            || self.reactor_temperature() > 175.0
            || self.separator_liquid_volume() / 35.3145 > 12.0
            || self.separator_liquid_volume() / 35.3145 < 1.0
            || self.stripper_liquid_volume() / 35.3145 > 8.0
            || self.stripper_liquid_volume() / 35.3145 < 1.0;

        self.set_shutdown_detected(if shutdown_detected { 1.0 } else { 0.0 });
    }
}
