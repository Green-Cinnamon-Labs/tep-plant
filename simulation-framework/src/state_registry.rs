// core/state_registry.rs
//
// StateRegistry (ver docs/issue55_opcua_refactor/plan_refactor.md, seções 1.3
// e 6). Ainda em transição: hoje só guarda CurrentState/EvaluationState e é
// singleton por construção (ver StateRegistry::shared()). Faltam implementar:
// a inscrição (subscribe — declarar outputs/inputs), resolve() (resolução em
// duas fases, validando que todo input declarado tem provedor) e
// set_current_state() (o commit EvaluationState -> CurrentState).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Uma entrada nomeada de um vetor: nome semântico + valor. A posição de um
/// slot dentro do `Vec` que o contém É o seu índice — não é redeclarada aqui.
///
/// Invariante: essas listas são append-only. Uma vez que um slot é registrado,
/// sua posição nunca muda nem é reaproveitada. Isso é o que permite um
/// consumidor resolver uma `key` para uma posição UMA ÚNICA VEZ e confiar
/// nessa posição para sempre.
pub struct StateSlot {
    pub key: String,
    pub value: f64,
}

pub struct StateRegistry {
    /// Estado oficial/persistido. `value` de cada slot aqui é o valor já
    /// confirmado do modelo.
    pub current_state: Vec<StateSlot>,

    /// Buffer de trabalho de uma rodada de avaliação (seção 8 do plano).
    /// Ainda incompleto: precisa de mutabilidade interior (por isso `Cell`,
    /// não `f64` puro) porque `evaluate()` é `&self` e escreve aqui via
    /// `Proxy` — que ainda não existe. `set_current_state()` é o commit
    /// `evaluation_state` -> `current_state` (também ainda não implementado).
    pub evaluation_state: Vec<Cell<f64>>,
}

impl StateRegistry {
    fn new() -> Self {
        Self { current_state: Vec::new(), evaluation_state: Vec::new() }
    }

    /// Único jeito de obter um StateRegistry — não existe construtor público
    /// que devolva um valor solto. `shared()` sempre embrulha em `Rc<RefCell<_>>`,
    /// então todo `DynamicModel` que se inscreve guarda um clone do mesmo `Rc`
    /// (barato — só incrementa o contador de referência), apontando pra a
    /// mesma instância. Isso é o que faz dele um singleton de fato: não é uma
    /// única instância *global*, é uma única instância *por simulação*,
    /// garantida pelo tipo — não por disciplina de quem usa.
    pub fn shared() -> Rc<RefCell<StateRegistry>> {
        Rc::new(RefCell::new(Self::new()))
    }
}
