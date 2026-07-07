// core/dynamic_model.rs
//
// Interface central do sistema de modelagem dinâmica.
//
// EM TRANSIÇÃO (ver docs/issue55_opcua_refactor/plan_refactor.md): o layout do
// estado (antes get_state_template()/StateTemplate) e a persistência (antes
// set_state()) saíram do DynamicModel — quem declara slots é a inscrição de
// cada DynamicModel em um StateRegistry singleton (core/state_registry.rs,
// inscrição ainda não implementada lá), que devolve Proxys; quem persiste é
// StateRegistry.set_current_state() (também ainda não implementado). Por isso
// DynamicModel só tem evaluate() por enquanto. evaluate() ainda usa a
// assinatura antiga (state: &[f64] -> EvaluationResult) até EvaluationState/
// Proxy existirem de fato — nesse ponto EvaluationResult também deixa de
// existir (evaluate() passa a escrever direto no EvaluationState via Proxy,
// sem retornar nada).

use crate::state_registry::StateSlot;

/// Resultado de evaluate() em um ponto de estado. `derivatives` continua um
/// `Vec<f64>` cru — é consumido posicionalmente pelo Integrator (k1..k4),
/// alinhado ao vetor de estado, e não precisa de nome semântico. `values` são
/// os valores intermediários calculados (ex.: "reactor.temperature"), como
/// slots nomeados.
pub struct EvaluationResult {
    pub derivatives: Vec<f64>,
    pub values: Vec<StateSlot>,
}

pub trait DynamicModel {
    fn name(&self) -> &'static str { "unnamed" }

    fn evaluate(&self, state: &[f64]) -> EvaluationResult;
}

/// Contrato de Composição: CompositeDynamicModel estende DynamicModel
/// (supertrait) — implementar esse trait exige implementar o outro também.
/// Só os DynamicModel que são nós compostos (ex.: TennesseeEastmanModel)
/// implementam isso. Componentes-folha (Valve, Agitator) não implementam —
/// tentar compô-los vira erro de compilação, não de runtime.
///
/// `add_dynamic` não declara slots nem funde template nenhum — quem declara
/// slots é a inscrição de cada DynamicModel direto no StateRegistry (fora
/// deste trait). O papel de `add_dynamic` é só ordenar: adiciona o
/// `evaluate()` do componente à sequência de avaliação do composto, na ordem
/// em que foi inserido. `models_mut` é o único método que cada composto
/// concreto precisa escrever — um getter trivial pro próprio
/// `Vec<Box<dyn DynamicModel>>`.
pub trait CompositeDynamicModel: DynamicModel {
    fn models_mut(&mut self) -> &mut Vec<Box<dyn DynamicModel>>;

    fn add_dynamic(&mut self, component: Box<dyn DynamicModel>) {
        self.models_mut().push(component);
    }
}
