/* Documentação: docs/07-controle.md */

#[monjolo::controller(name = "separator_level_control")]
pub struct SeparatorLevelControl {
    #[sensor(key = "xmeas.separator.level")]
    level: f64,
    #[actuator(key = "valve.separator_underflow.position")]
    underflow: f64,
    #[state(key = "controller.separator_level.integral")]
    integral: f64,
}

impl SeparatorLevelControl {
    fn control(&self) -> f64 {
        const KC: f64 = 1.0;
        const TI_HOURS: f64 = 200.0 / 60.0;
        const SETPOINT: f64 = 50.0;
        const BIAS: f64 = 38.1;

        let error = self.level().read() - SETPOINT;
        let raw = BIAS + KC * (error + self.integral() / TI_HOURS);
        self.underflow().write(raw.clamp(0.0, 100.0));

        let winding_up = (raw > 100.0 && KC * error > 0.0) || (raw < 0.0 && KC * error < 0.0);
        if winding_up { 0.0 } else { error }
    }
}
