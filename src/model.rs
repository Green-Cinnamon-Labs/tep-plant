/** tep/model.rs

Monta o `Composite` (`monjolo::dynamic_model`) da planta TEP — os 4 subsistemas químicos
(Flows/Heat/Measurements ainda não migrados, ver flows.rs) mais os dois atuadores físicos: a válvula
de água de resfriamento do reator e o agitador. Cada um se inscreve no StateRegistry recebido
durante sua própria construção; quem chama `new()` ainda precisa chamar `registry.resolve()` depois
— esta função não decide isso.

NOTA (2026-07-30): não existe mais uma struct `TennesseeEastmanModel`. `Composite` já implementa
`DynamicModel`/`CompositeDynamicModel` sozinho — não sobra nenhum dado/comportamento específico do
TEP que justifique uma struct própria, então `new()` é só uma função livre que monta e devolve o
`Composite` já pronto, nomeado via `.named()`. `Valve`/`Agitator` são as duas coisas ao mesmo tempo:
`DynamicModel` (física real, `add_dynamic`'d) *e* `Actuator` (aceitam comando via `write()`) — não
há caixa de correio separada.

A construção de `Sensor`s (e a camada de descoberta externa — quais sensores/atuadores ficam
visíveis pra um adaptador de rede) foi retirada daqui por enquanto, pendente de redesenho: sem
struct própria, não há mais onde mantê-los vivos.

`new()` recebe um `&Snapshot` (condição inicial já carregada por quem chama — não é responsabilidade
do modelo saber de onde vem o arquivo) e repassa pra cada subsistema buscar só as chaves que
interessam pra ele (ver Reactor::new/Separator::new/Stripper::new/Compressor::new).
*/

use monjolo::dynamic_model::{Composite, CompositeDynamicModel};
use monjolo::snapshot::Snapshot;
use monjolo::state_registry::StateRegistry;

use crate::subsystems::actuators::{Agitator, Valve};
use crate::subsystems::compressor::Compressor;
use crate::subsystems::reactor::Reactor;
use crate::subsystems::separator::Separator;
use crate::subsystems::stripper::Stripper;

/** `VTAU(10)` em `teprob.f`: constante de tempo da válvula de água de resfriamento do reator — 5
segundos, convertidos pra horas (unidade que o resto da física do TEP usa, ver
`Simulation::set_dt_hours`).
*/
const COOLING_WATER_VALVE_TAU_HOURS: f64 = 5.0 / 3600.0;

/** `VTAU(12)` em `teprob.f`: constante de tempo do agitador — também 5 segundos.
*/
const AGITATOR_TAU_HOURS: f64 = 5.0 / 3600.0;

pub fn new(registry: &mut StateRegistry, initial: &Snapshot) -> Composite {
    
    let mut composite = Composite::new().named("TennesseeEastmanModel");

    composite.add_dynamic(Box::new(Reactor::new(registry, initial)));
    composite.add_dynamic(Box::new(Separator::new(registry, initial)));
    composite.add_dynamic(Box::new(Stripper::new(registry, initial)));
    composite.add_dynamic(Box::new(Compressor::new(registry, initial)));
    composite.add_dynamic(Box::new(Valve::new(
        registry,
        "cooling_water",
        COOLING_WATER_VALVE_TAU_HOURS,
    )));
    composite.add_dynamic(Box::new(Agitator::new(registry, AGITATOR_TAU_HOURS)));

    composite
}

#[cfg(test)]
mod tests {
    use super::*;
    use monjolo::dynamic_model::DynamicModel;

    /** Prova que a cadeia inteira funciona: construção + inscrição (add_dynamic já dispara
    subscribe() dentro de cada Reactor/Separator/Stripper/Compressor/Valve/Agitator::new()) +
    resolve() (sem erro) + evaluate() sem panic (nenhum Proxy é lido antes de ser resolvido).
    */
    #[test]
    fn wires_and_evaluates_without_panicking() {
        let registry = StateRegistry::shared();
        let initial = Snapshot::from_pairs(&[]);
        let tep = new(&mut registry.borrow_mut(), &initial);

        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        tep.evaluate();
    }
}
