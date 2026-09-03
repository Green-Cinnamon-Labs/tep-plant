/* tep/sensors/separator_underflow.rs */

/* XMEAS(14), Separator Underflow (Stream 10) — m³/hr, publicado por Measured. */
#[monjolo::sensor(key = "xmeas.stream10.flow_rate")]
pub struct SeparatorUnderflow;
