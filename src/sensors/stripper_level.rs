/* tep/sensors/stripper_level.rs */

/* Análogo a XMEAS(15), Stripper Liquid Level (%) — `xmeas.stripper.level`, mesma razão de
sensors/separator_level.rs: a malha opera sobre o valor já convertido pra % que `Measured` publica,
não `stripper.liquid_volume` bruto.
*/
#[monjolo::sensor(key = "xmeas.stripper.level")]
pub struct StripperLevel;
