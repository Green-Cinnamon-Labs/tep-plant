/* Documentação: docs/07-controle.md */

#[monjolo::controller(name = "stripper_level_control")]
pub struct StripperLevelControl {
    #[sensor(key = "xmeas.stripper.level")]
    level: f64,
    #[actuator(key = "valve.stripper_product.position")]
    product: f64,
    #[state(key = "controller.stripper_level.integral")]
    integral: f64,
}

impl StripperLevelControl {
    fn control(&self) -> f64 {
        const KC: f64 = 1.0;
        const TI_HOURS: f64 = 200.0 / 60.0;
        const SETPOINT: f64 = 50.0;
        const BIAS: f64 = 46.5;

        let error = self.level().read() - SETPOINT;
        let raw = BIAS + KC * (error + self.integral() / TI_HOURS);
        self.product().write(raw.clamp(0.0, 100.0));

        let winding_up = (raw > 100.0 && KC * error > 0.0) || (raw < 0.0 && KC * error < 0.0);
        if winding_up { 0.0 } else { error }
    }
}
