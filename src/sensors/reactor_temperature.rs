/* tep/sensors/reactor_temperature.rs */

/* Análogo a XMEAS(9), Reactor Temperature. */
#[monjolo::sensor(key = "reactor.temperature")]
pub struct ReactorTemperature;
