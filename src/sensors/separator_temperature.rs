/* tep/sensors/separator_temperature.rs */

/* Análogo a XMEAS(11), Product Separator Temperature. */
#[monjolo::sensor(key = "separator.temperature")]
pub struct SeparatorTemperature;
