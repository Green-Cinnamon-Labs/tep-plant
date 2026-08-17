/* tep/sensors/stripper_temperature.rs */

/* Análogo a XMEAS(18), Stripper Temperature. */
#[monjolo::sensor(key = "stripper.temperature")]
pub struct StripperTemperature;
