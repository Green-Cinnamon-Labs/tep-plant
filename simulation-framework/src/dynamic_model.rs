// core/dynamic_model.rs
//
// Interface central do sistema de modelagem dinâmica.
//
// O layout do estado (antes get_state_template()/StateTemplate) e a
// persistência (antes set_state()) saíram do DynamicModel — quem declara
// slots é a inscrição de cada DynamicModel em um StateRegistry singleton
// (state_registry.rs), via subscribe(), que devolve Proxys; quem persiste é
// StateRegistry.set_current_state() (ainda não implementado). evaluate() não
// devolve mais nada — inclusive as derivadas (antes um Vec<f64> cru à parte)
// agora são só mais um output endereçado por Proxy, escrito no
// EvaluationState igual qualquer outro valor (ver plan_refactor.md, seção
// 8.3). EvaluationResult foi eliminado.

use crate::state_registry::EvaluationState;

pub trait DynamicModel {
    fn name(&self) -> &'static str { "unnamed" }

    fn evaluate(&self, state: &[f64], eval: &EvaluationState);
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
