/* tep/sensors/stripper_underflow.rs */

/* XMEAS(17), Stripper Underflow (Stream 11, produto final) — m³/hr, publicado por Measured. */
#[monjolo::sensor(key = "xmeas.stream11.flow_rate", noise = 0.115)]
pub struct StripperUnderflow;
