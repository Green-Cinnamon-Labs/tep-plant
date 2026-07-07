// simulation-framework/simulation.rs
//
// Interface externa do framework — o que uma planta (ex.: TennesseeEastmanModel)
// usa pra rodar de verdade. Tudo em dynamic_model.rs/state_registry.rs/method/
// actuator/sensor/disturbance é implementação interna; Simulation é a fachada
// pública que junta o StateRegistry com o resto (ver
// docs/issue55_opcua_refactor/plan_refactor.md, seção 9).
//
// EM TRANSIÇÃO: falta Proxy, StateRegistry::subscribe()/resolve(), o
// EvaluationState de verdade (hoje é só um Vec<Cell<f64>> cru dentro do
// StateRegistry) e Integrator::step() com a assinatura nova (closure em vez
// de `&mut dyn DynamicModel` — Integrator/RK4 ainda estão comentados, com a
// assinatura antiga). Por enquanto Simulation só segura o que já existe de
// verdade: o composto raiz e o StateRegistry compartilhado.

use std::cell::RefCell;
use std::rc::Rc;

use crate::dynamic_model::DynamicModel;
use crate::state_registry::StateRegistry;

pub struct Simulation {
    model: Box<dyn DynamicModel>,
    registry: Rc<RefCell<StateRegistry>>,
}

impl Simulation {
    pub fn new(model: Box<dyn DynamicModel>) -> Self {
        Self { model, registry: StateRegistry::shared() }
    }
}
