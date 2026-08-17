/* tep/controllers/mod.rs */

/** Controllers da planta, um arquivo por controller (mesma convenção de actuators/sensors) — usa
`#[controller(...)]` (`monjolo-macros`): se auto-registra via `inventory::submit!` escondido;
nenhum `build_tep()` (nem `main()`) precisa conhecer o tipo. Sem lógica de controle nenhuma ainda
(nem `step()`/`update()`, nem PID, nem setpoint/ganho) — só a declaração de dependências de
Sensor/Actuator (Controller ainda é "open item", ver CONTRIBUTING.md); `evaluate()` (fase C da
árvore de avaliação) é vazio de propósito, definido uma única vez em
`monjolo::controller::model::Controller`.
*/

mod reactor_pressure_control;

pub use reactor_pressure_control::ReactorPressureControl;
