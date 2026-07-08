// tep/model.rs
//
// TennesseeEastmanModel — implementação concreta de DynamicModel para a planta
// TEP. Reconstrução do zero sobre o contrato atual (ver
// docs/issue55_opcua_refactor/plan_refactor.md, seções 1-2). A versão
// anterior, com toda a química/distúrbio já resolvidos por dynamics(), está
// preservada e comentada em _deprecated.rs.
//
// new() é uma sequência de add_dynamic (seção 2.6) — só os 4 componentes que
// já têm evaluate() real (Flows/Heat/Measurements/Valve/Agitator ainda não
// migrados, ver flows.rs). Cada um se inscreve no StateRegistry recebido
// durante sua própria construção; quem chama new() ainda precisa chamar
// `registry.resolve()` depois — TennesseeEastmanModel não decide isso.

use simulation_framework::dynamic_model::{CompositeDynamicModel, DynamicModel};
use simulation_framework::state_registry::StateRegistry;

use crate::compressor::Compressor;
use crate::reactor::Reactor;
use crate::separator::Separator;
use crate::stripper::Stripper;

pub struct TennesseeEastmanModel {
    models: Vec<Box<dyn DynamicModel>>,
}

impl TennesseeEastmanModel {
    pub fn new(registry: &mut StateRegistry) -> Self {
        let mut tep = Self { models: Vec::new() };
        tep.add_dynamic(Box::new(Reactor::new(registry)));
        tep.add_dynamic(Box::new(Separator::new(registry)));
        tep.add_dynamic(Box::new(Stripper::new(registry)));
        tep.add_dynamic(Box::new(Compressor::new(registry)));
        tep
    }
}

impl DynamicModel for TennesseeEastmanModel {
    fn name(&self) -> &'static str {
        "TennesseeEastmanModel"
    }

    fn evaluate(&self) {
        self.evaluate_children();
    }
}

impl CompositeDynamicModel for TennesseeEastmanModel {
    fn models(&self) -> &[Box<dyn DynamicModel>] {
        &self.models
    }

    fn models_mut(&mut self) -> &mut Vec<Box<dyn DynamicModel>> {
        &mut self.models
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prova que a cadeia inteira funciona: construção + inscrição (add_dynamic
    /// já dispara subscribe() dentro de cada Reactor/Separator/Stripper/
    /// Compressor::new()) + resolve() (sem erro — reactor.temperature e
    /// separator.temperature têm provedor) + evaluate() sem panic (nenhum
    /// Proxy é lido antes de ser resolvido).
    #[test]
    fn wires_and_evaluates_without_panicking() {
        let registry = StateRegistry::shared();
        let tep = TennesseeEastmanModel::new(&mut registry.borrow_mut());

        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        tep.evaluate();
    }
}
