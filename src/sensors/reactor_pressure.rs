/* tep/sensors/reactor_pressure.rs */

/* Análogo a XMEAS(7), Reactor Pressure — `xmeas.reactor.pressure` (kPa, já convertido por
`Measured`: `(reactor.pressure - 760)/760*101.325`), não `reactor.pressure` bruto: esse último é a
grandeza interna do solver físico, em mmHg (`GAS_CONSTANT = 998.9 mmHg·m³/(kmol·K)` em reactor.rs) —
um transmissor de pressão de verdade nunca reportaria isso direto, só o valor já calibrado/
convertido pro padrão XMEAS. Descoberto porque `ReactorPressureControl` (Kp=0.1, setpoint=2705)
saturava a válvula de purge com a leitura em mmHg (~20000+) em vez de kPa (~2700).
*/
#[monjolo::sensor(key = "xmeas.reactor.pressure")]
pub struct ReactorPressure;
