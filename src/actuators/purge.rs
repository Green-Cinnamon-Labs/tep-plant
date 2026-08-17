/* tep/actuators/purge.rs */

/* XMV-6: Purge Valve. VTAU(6) = 5s. */
#[monjolo::actuator(key = "valve.purge.position")]
pub struct Purge {
    #[command]
    command: f64,
    #[state]
    position: f64,
}

impl Purge {
    fn dynamics(&self) -> f64 {
        let tau = 5.0 / 3600.0;
        (self.command() - self.position()) / tau
    }
}
