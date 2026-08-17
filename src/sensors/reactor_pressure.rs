/* tep/sensors/reactor_pressure.rs */

/* Análogo a XMEAS(7), Reactor Pressure. */
#[monjolo::sensor(key = "reactor.pressure")]
pub struct ReactorPressure;
