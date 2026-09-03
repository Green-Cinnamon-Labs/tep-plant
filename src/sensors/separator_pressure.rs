/* tep/sensors/separator_pressure.rs */

/* XMEAS(13), Product Separator Pressure (kPa gauge) — `xmeas.separator.pressure`, já convertido
por Measured (`(separator.pressure - 760)/760*101.325`), não `separator.pressure` bruto: esse
último é a grandeza interna do solver físico, em mmHg — mesma razão de sensors/reactor_pressure.rs.
*/
#[monjolo::sensor(key = "xmeas.separator.pressure")]
pub struct SeparatorPressure;
