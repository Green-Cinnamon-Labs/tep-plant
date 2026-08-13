/** tep/model.rs

Monta o `Composite` (`monjolo::dynamic_model`) da planta TEP — os 4 subsistemas químicos
(Flows/Heat/Measurements ainda não migrados, ver flows.rs) mais os 12 atuadores físicos (XMV-1 a
XMV-12), 6 sensores (escopo representativo) e 1 controller. Nenhum atuador/sensor/controller tem
tipo Rust próprio (`monjolo::actuator::model::Actuator`/`sensor::model::Sensor`/
`controller::model::Controller`, todos genéricos) — a construção de cada um mora em
`subsystems/{actuators,sensors,controllers}.rs`, uma função por componente; aqui só se chama cada
uma. Cada um se inscreve no StateRegistry recebido durante sua própria construção; quem chama
`build_tep()` ainda precisa chamar `registry.resolve()` depois — esta função não decide isso.

NOTA (2026-07-30): não existe mais uma struct `TennesseeEastmanModel`. `Composite` já implementa
`DynamicModel`/`CompositeDynamicModel` sozinho — não sobra nenhum dado/comportamento específico do
TEP que justifique uma struct própria, então `build_tep()` é só uma função livre que monta e devolve
o `Composite` já pronto, nomeado via `.named()`. Cada atuador é as duas coisas ao mesmo tempo:
`DynamicModel` (física real, `add_dynamic`'d) *e* `Actuator` (aceita comando via `write()`) — não há
caixa de correio separada.

NOTA (2026-08-10): `Sensor`/`Actuator`/`Controller` se catalogam sozinhos, na própria construção —
`Sensor::new()`/`Actuator::new()`/`Controller::new()` já chamam `offer_sensor()`/`offer_actuator()`/
`offer_controller()` internamente, sob a própria chave (ou `name`, no caso do Controller), e
devolvem `Arc<Sensor>`/`Rc<Actuator>`/`Rc<Controller>` — o mesmo ponteiro que guardaram no catálogo.
Por isso não há mais chamada manual a `offer_*` aqui: cada função de `subsystems/actuators.rs`,
`sensors.rs` ou `controllers.rs` já devolve algo pronto pra usar (`add_dynamic` pro Actuator;
simplesmente descartável pro Sensor/Controller, já que o catálogo do StateRegistry guarda a
referência que importa). `Sensor`/`Controller` nunca são `add_dynamic`'d — ficam fora da árvore do
composto, não participam de `evaluate()`.

`build_tep()` recebe um `&Snapshot` (condição inicial já carregada por quem chama — não é
responsabilidade do modelo saber de onde vem o arquivo) e repassa pra cada subsistema buscar só as
chaves que interessam pra ele (ver Reactor::new/Separator::new/Stripper::new/Compressor::new).
*/

use monjolo::dynamic_model::{Composite, CompositeDynamicModel};
use monjolo::snapshot::Snapshot;
use monjolo::state_registry::StateRegistry;

use crate::subsystems::actuators;
use crate::subsystems::compressor::Compressor;
use crate::subsystems::controllers;
use crate::subsystems::reactor::Reactor;
use crate::subsystems::sensors;
use crate::subsystems::separator::Separator;
use crate::subsystems::stripper::Stripper;

pub fn build_tep(registry: &mut StateRegistry, initial: &Snapshot) -> Composite {
    
    let mut composite = Composite::new().named("TennesseeEastmanModel");

    // Subsistemas com termodinâmica
    composite.add_dynamic(Box::new(Reactor::new(registry, initial)));
    composite.add_dynamic(Box::new(Separator::new(registry, initial)));
    composite.add_dynamic(Box::new(Stripper::new(registry, initial)));
    composite.add_dynamic(Box::new(Compressor::new(registry, initial)));

    // Atuadores (XMV-1 a XMV-12, ordem canônica do cabeçalho de teprob.f) — cada função em
    // subsystems/actuators.rs constrói e já cataloga uma instância de
    // monjolo::actuator::model::Actuator, sob a própria chave.
    composite.add_dynamic(Box::new(actuators::feed_d(registry)));
    composite.add_dynamic(Box::new(actuators::feed_e(registry)));
    composite.add_dynamic(Box::new(actuators::feed_a(registry)));
    composite.add_dynamic(Box::new(actuators::feed_ac(registry)));
    composite.add_dynamic(Box::new(actuators::compressor_recycle(registry)));
    composite.add_dynamic(Box::new(actuators::purge(registry)));
    composite.add_dynamic(Box::new(actuators::separator_underflow(registry)));
    composite.add_dynamic(Box::new(actuators::stripper_product(registry)));
    composite.add_dynamic(Box::new(actuators::stripper_steam(registry)));
    composite.add_dynamic(Box::new(actuators::reactor_cooling_water(registry)));
    composite.add_dynamic(Box::new(actuators::condenser_cooling_water(registry)));
    composite.add_dynamic(Box::new(actuators::agitator(registry)));

    // Sensores (escopo representativo — ver subsystems/sensors.rs) — construir já cataloga; não
    // há árvore de composto pra Sensor entrar (não é DynamicModel), então nada mais a fazer aqui.
    sensors::reactor_temperature(registry);
    sensors::reactor_pressure(registry);
    sensors::separator_temperature(registry);
    sensors::separator_pressure(registry);
    sensors::stripper_temperature(registry);
    sensors::compressor_pressure(registry);

    // Controladores (escopo representativo — ver subsystems/controllers.rs) — mesma ideia: nomes
    // de Sensor/Actuator que precisa são resolvidos depois, não importa a ordem (Art. 6.3).
    controllers::reactor_pressure_control(registry);

    composite
}

#[cfg(test)]
mod tests {
    use super::*;
    use monjolo::dynamic_model::DynamicModel;

    /** Prova que a cadeia inteira funciona: construção + inscrição (add_dynamic/Sensor::new()/
    Controller::new() já disparam subscribe()/subscribe_read()/need_*() dentro de cada componente)
    + resolve() (sem erro) + evaluate() sem panic (nenhum Proxy é lido antes de ser resolvido).
    */
    #[test]
    fn wires_and_evaluates_without_panicking() {
        let registry = StateRegistry::shared();
        let initial = Snapshot::from_pairs(&[]);
        let tep = build_tep(&mut registry.borrow_mut(), &initial);

        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        tep.evaluate();
    }
}
