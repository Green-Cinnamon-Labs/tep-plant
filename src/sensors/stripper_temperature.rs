/* tep/sensors/stripper_temperature.rs */

/* XMEAS(18), Stripper Temperature (°C) — `xmeas.stripper.temperature`, publicado por Measured,
não `stripper.temperature` bruto (mesma razão de reactor_temperature.rs).
*/
#[monjolo::sensor(key = "xmeas.stripper.temperature")]
pub struct StripperTemperature;
