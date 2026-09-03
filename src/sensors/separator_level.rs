/* tep/sensors/separator_level.rs */

/* Análogo a XMEAS(12), Separator Liquid Level (%) — `xmeas.separator.level`, não
`separator.liquid_volume`: a malha de controle de nível opera sobre o valor já convertido pra % que
`Measured` publica (Block 21 de teprob.f: `(volume - offset) / range * 100`), a mesma grandeza que um
transmissor de nível de verdade reportaria, não o volume bruto em m³.
*/
#[monjolo::sensor(key = "xmeas.separator.level")]
pub struct SeparatorLevel;
