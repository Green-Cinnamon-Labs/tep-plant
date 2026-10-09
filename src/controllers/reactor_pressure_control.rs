/* Documentação: docs/07-controle.md */

#[monjolo::controller(name = "reactor_pressure_control")]
pub struct ReactorPressureControl {
    #[sensor(key = "xmeas.reactor.pressure")]
    pressure: f64,
    #[actuator(key = "valve.purge.position")]
    purge: f64,
    #[state(key = "controller.reactor_pressure.integral")]
    integral: f64,
}

impl ReactorPressureControl {
    fn control(&self) -> f64 {
        const KC: f64 = 0.1;
        const TI_HOURS: f64 = 20.0 / 60.0;
        const SETPOINT: f64 = 2705.0;
        const BIAS: f64 = 40.06;

        let error = self.pressure().read() - SETPOINT;
        let raw = BIAS + KC * (error + self.integral() / TI_HOURS);
        self.purge().write(raw.clamp(0.0, 100.0));

        let winding_up = (raw > 100.0 && KC * error > 0.0) || (raw < 0.0 && KC * error < 0.0);
        if winding_up { 0.0 } else { error }
    }
}
