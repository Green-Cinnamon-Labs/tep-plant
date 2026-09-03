/* tep/sensors/stripper_pressure.rs */

/* XMEAS(16), Stripper Pressure (kPa gauge) — `xmeas.stripper.pressure`, publicado por Measured.
Fiel ao teprob.f original: reusa a pressão do compressor (PTV) — o modelo clássico não tem um
estado de pressão próprio pro vaso do stripper (teprob.f:977, `XMEAS(16) = (PTV-760)/760*101.325`).
*/
#[monjolo::sensor(key = "xmeas.stripper.pressure")]
pub struct StripperPressure;
