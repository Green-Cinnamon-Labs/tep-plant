/* tep/controllers/reactor_pressure_control.rs */

/* XMEAS(7) Reactor Pressure -> XMV(6) Purge Valve, o laço de pressão do reator
(docs/07-controle.md: setpoint 2705 kPa, Kp=0.1 — não usados ainda, sem lógica de controle).

Só este par está aqui por enquanto: é o único dos 3 laços clássicos do TEP (docs/07-controle.md)
cuja medição já existe de verdade hoje (`reactor.pressure`, ver sensors/reactor_pressure.rs) —
separador/stripper precisariam de "nível" em %, e só temos volume
(`separator.liquid_volume`/`stripper.liquid_volume`), grandeza fisicamente diferente, não um
substituto direto.

Nomes de Sensor/Actuator abaixo são as chaves reais do StateRegistry (`"reactor.pressure"`,
`"valve.purge.position"`), não apelidos curtos — `Sensor`/`Actuator` se catalogam sob a própria
chave, não existe nome de catálogo separado.
*/
#[monjolo::controller(
    name = "reactor_pressure_control",
    sensors = ["reactor.pressure"],
    actuators = ["valve.purge.position"]
)]
pub struct ReactorPressureControl;
