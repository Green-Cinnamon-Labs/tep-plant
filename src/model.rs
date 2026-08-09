/** tep/model.rs

Monta o `Composite` (`monjolo::dynamic_model`) da planta TEP — os 4 subsistemas químicos
(Flows/Heat/Measurements ainda não migrados, ver flows.rs) mais os 12 atuadores físicos: as 11
válvulas (XMV-1 a XMV-11) e o agitador (XMV-12), cada um uma instância de
`monjolo::actuator::model::Actuator` — não há mais um tipo Rust por atuador (`FeedDValve` etc. não
existem). A construção de cada um (chave + lei física) mora em `subsystems/actuators.rs`, uma
função por atuador; aqui só se chama cada uma e `add_dynamic`. Cada um se inscreve no StateRegistry
recebido durante sua própria construção; quem chama `build_tep()` ainda precisa chamar
`registry.resolve()` depois — esta função não decide isso.

NOTA (2026-07-30): não existe mais uma struct `TennesseeEastmanModel`. `Composite` já implementa
`DynamicModel`/`CompositeDynamicModel` sozinho — não sobra nenhum dado/comportamento específico do
TEP que justifique uma struct própria, então `build_tep()` é só uma função livre que monta e devolve
o `Composite` já pronto, nomeado via `.named()`. Cada atuador é as duas coisas ao mesmo tempo:
`DynamicModel` (física real, `add_dynamic`'d) *e* `Actuator` (aceita comando via `write()`) — não há
caixa de correio separada.

A construção de `Sensor`s (e a camada de descoberta externa — quais sensores/atuadores ficam
visíveis pra um adaptador de rede) foi retirada daqui por enquanto, pendente de redesenho: sem
struct própria, não há mais onde mantê-los vivos.

`build_tep()` recebe um `&Snapshot` (condição inicial já carregada por quem chama — não é
responsabilidade do modelo saber de onde vem o arquivo) e repassa pra cada subsistema buscar só as
chaves que interessam pra ele (ver Reactor::new/Separator::new/Stripper::new/Compressor::new).
*/

use monjolo::dynamic_model::{Composite, CompositeDynamicModel};
use monjolo::snapshot::Snapshot;
use monjolo::state_registry::StateRegistry;

use crate::subsystems::actuators;
use crate::subsystems::compressor::Compressor;
use crate::subsystems::reactor::Reactor;
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
    // subsystems/actuators.rs constrói uma instância de monjolo::actuator::model::Actuator.
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

    // Sensores

    // Controladores

    composite
}

#[cfg(test)]
mod tests {
    use super::*;
    use monjolo::dynamic_model::DynamicModel;

    /** Prova que a cadeia inteira funciona: construção + inscrição (add_dynamic já dispara
    subscribe() dentro de cada Reactor/Separator/Stripper/Compressor/Actuator::new()) + resolve()
    (sem erro) + evaluate() sem panic (nenhum Proxy é lido antes de ser resolvido).
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
