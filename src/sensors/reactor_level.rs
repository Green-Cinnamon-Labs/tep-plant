/* tep/sensors/reactor_level.rs */

/* XMEAS(8), Reactor Level (%) — publicado por Measured (já convertido de volume, Block 21 de
teprob.f), não `reactor.liquid_volume` bruto.
*/
#[monjolo::sensor(key = "xmeas.reactor.level")]
pub struct ReactorLevel;
