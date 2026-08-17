/* tep/sensors/separator_pressure.rs */

/* Análogo a XMEAS(13), Product Separator Pressure. */
#[monjolo::sensor(key = "separator.pressure")]
pub struct SeparatorPressure;
