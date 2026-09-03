/* tep/sensors/separator_temperature.rs */

/* XMEAS(11), Product Separator Temperature (°C) — `xmeas.separator.temperature`, publicado por
Measured, não `separator.temperature` bruto (mesma razão de reactor_temperature.rs).
*/
#[monjolo::sensor(key = "xmeas.separator.temperature")]
pub struct SeparatorTemperature;
