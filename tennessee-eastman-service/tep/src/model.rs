// tep/model.rs
//
// TennesseeEastmanModel — implementação concreta de DynamicModel para a planta
// TEP. Reconstrução do zero sobre o contrato atual (ver
// docs/issue55_opcua_refactor/plan_refactor.md, seções 1-2). A versão
// anterior, com toda a química/distúrbio já resolvidos por dynamics(), está
// preservada e comentada em _deprecated.rs — nada de lá foi migrado ainda de
// propósito.

use simulation_framework::dynamic_model::{CompositeDynamicModel, DynamicModel};
use simulation_framework::state_registry::EvaluationState;

pub struct TennesseeEastmanModel {
    models: Vec<Box<dyn DynamicModel>>,
}

impl TennesseeEastmanModel {
    pub fn new() -> Self {
        Self { models: Vec::new() }
    }
}

impl DynamicModel for TennesseeEastmanModel {
    fn name(&self) -> &'static str {
        "TennesseeEastmanModel"
    }

    fn evaluate(&self, _state: &[f64], _eval: &EvaluationState) {
        todo!("ainda não há componentes registrados nem química ligada")
    }
}

impl CompositeDynamicModel for TennesseeEastmanModel {
    fn models_mut(&mut self) -> &mut Vec<Box<dyn DynamicModel>> {
        &mut self.models
    }
}
