/* tep/sensors/reactor_temperature.rs */

/* XMEAS(9), Reactor Temperature (°C) — `xmeas.reactor.temperature`, publicado por Measured, não
`reactor.temperature` bruto (mesma grandeza física aqui, mas a chave XMEAS é a que representa o
que um transmissor de verdade reportaria — consistente com o resto de sensors/, ver
reactor_pressure.rs pra um caso onde a diferença numérica importa de verdade).
*/
#[monjolo::sensor(key = "xmeas.reactor.temperature")]
pub struct ReactorTemperature;
