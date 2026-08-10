/* tep/subsystems/controllers.rs */

/** Controllers da planta — cada um é uma instância de `monjolo::controller::model::Controller`
(nomes de Sensor/Actuator que precisa, resolvidos via `need_sensor()`/`need_actuator()` do
StateRegistry), sem lógica de controle nenhuma ainda: nem `step()`/`update()`, nem PID, nem
setpoint/ganho — só a declaração de dependências (Controller ainda é "open item", ver
CONTRIBUTING.md). A malha em si fica pra quando esse design existir.

Só o par reator (pressão → purga) está aqui por enquanto: é o único dos 3 laços clássicos do TEP
(docs/07-controle.md) cuja medição já existe de verdade hoje (`reactor.pressure`, ver
subsystems/sensors.rs) — separador/stripper precisariam de "nível" em %, e só temos volume
(`separator.liquid_volume`/`stripper.liquid_volume`), grandeza fisicamente diferente, não um
substituto direto.

Nomes de Sensor/Actuator aqui são as chaves reais do StateRegistry (`"reactor.pressure"`,
`"valve.purge.position"`), não apelidos curtos — `Sensor`/`Actuator` se catalogam sob a própria
chave (ver subsystems/sensors.rs, subsystems/actuators.rs), não existe mais um nome de catálogo
separado. `Controller::new()` já registra o controller no catálogo do StateRegistry sob `name` —
devolve `Rc<Controller>`, não `Controller`.
*/

use std::rc::Rc;

use monjolo::controller::model::Controller;
use monjolo::state_registry::StateRegistry;

/* XMEAS(7) Reactor Pressure -> XMV(6) Purge Valve, o laço de pressão do reator
(docs/07-controle.md: setpoint 2705 kPa, Kp=0.1 — não usados ainda, sem lógica de controle).
*/
pub fn reactor_pressure_control(registry: &mut StateRegistry) -> Rc<Controller> {
    Controller::new(
        registry,
        "reactor_pressure_control",
        &["reactor.pressure"],
        &["valve.purge.position"],
    )
}
