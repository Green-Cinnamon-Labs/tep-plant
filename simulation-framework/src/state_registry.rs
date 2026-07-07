/**simulation-framework/state_registry.rs

StateRegistry (ver docs/issue55_opcua_refactor/plan_refactor.md, seções 1.3,
6 e 7). Ainda em transição: falta set_current_state() (o commit
EvaluationState -> CurrentState).
*/
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/**Uma entrada nomeada de um vetor: nome semântico + valor. A posição de um
 slot dentro do `Vec` que o contém É o seu índice — não é redeclarada aqui.

 Invariante: essas listas são append-only. Uma vez que um slot é registrado,
 sua posição nunca muda nem é reaproveitada. Isso é o que permite um
 consumidor resolver uma `key` para uma posição UMA ÚNICA VEZ e confiar
 nessa posição para sempre.
*/
pub struct StateSlot {
    pub key: String,
    pub value: f64,
}

/** Handle compartilhado pra uma posição em `evaluation_state`. Nasce sem
resolução (`index = usize::MAX`); `StateRegistry::resolve()` escreve o
índice real nele. Todo clone de um `Proxy` aponta pro mesmo `Cell`, então
resolver uma vez basta — o componente guarda seu clone desde a inscrição e
nunca mais precisa perguntar pelo nome de novo.

Agnóstico a se o valor por trás é "hipotético" (chute intermediário de um
solver iterativo) ou "real" (convergido) — só endereça a posição.
*/
#[derive(Clone)]
pub struct Proxy {
    index: Rc<Cell<usize>>,
}

impl Proxy {
    fn resolved(index: usize) -> Self {
        Self { index: Rc::new(Cell::new(index)) }
    }

    fn unresolved() -> Self {
        Self { index: Rc::new(Cell::new(usize::MAX)) }
    }

    pub fn get(&self) -> usize {
        let idx = self.index.get();
        debug_assert!(idx != usize::MAX, "Proxy usado antes de StateRegistry::resolve()");
        idx
    }
}

/** View tipada sobre o `evaluation_state` cru de um `StateRegistry` (seção 8
do plano). `evaluate()` recebe isso em vez do `Vec<Cell<f64>>` bruto — lê e
escreve sempre através de um `Proxy` já resolvido, nunca por nome.
*/
pub struct EvaluationState<'a> {
    buf: &'a [Cell<f64>],
}

impl<'a> EvaluationState<'a> {
    pub fn new(buf: &'a [Cell<f64>]) -> Self {
        Self { buf }
    }

    pub fn get(&self, proxy: &Proxy) -> f64 {
        self.buf[proxy.get()].get()
    }

    pub fn set(&self, proxy: &Proxy, value: f64) {
        self.buf[proxy.get()].set(value);
    }
}

pub struct StateRegistry {
    /// Estado oficial/persistido. `value` de cada slot aqui é o valor já
    /// confirmado do modelo.
    pub current_state: Vec<StateSlot>,

    /// Buffer de trabalho de uma rodada de avaliação (seção 8 do plano).
    /// `Cell`, não `f64` puro, porque `evaluate()` é `&self` e escreve aqui
    /// via `Proxy`.
    pub evaluation_state: Vec<Cell<f64>>,

    /// nome semântico -> posição em `evaluation_state`, preenchido conforme
    /// os outputs vão sendo oferecidos em subscribe().
    index: HashMap<String, usize>,

    /// Inputs declarados em subscribe(), ainda não resolvidos. resolve()
    /// esvazia essa lista, escrevendo a posição real em cada Proxy.
    pending_requests: Vec<(String, Proxy)>,
}

impl StateRegistry {
    fn new() -> Self {
        Self {
            current_state: Vec::new(),
            evaluation_state: Vec::new(),
            index: HashMap::new(),
            pending_requests: Vec::new(),
        }
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

    pub fn evaluation_state(&self) -> EvaluationState<'_> {
        EvaluationState::new(&self.evaluation_state)
    }

    /// Um DynamicModel se inscreve: `offers` são os nomes dos slots que ele
    /// próprio provê (reservados e resolvidos na hora — a posição já é
    /// conhecida no momento em que a posição é criada); `needs` são as chaves
    /// de outros componentes que ele vai ler (devolvidas como Proxy NÃO
    /// resolvido — só ganham posição real em resolve()). Não importa a ordem
    /// de inscrição entre quem oferece e quem pede.
    pub fn subscribe(&mut self, offers: &[&str], needs: &[&str]) -> (Vec<Proxy>, Vec<Proxy>) {
        let offered = offers.iter().map(|&key| {
            let idx = self.evaluation_state.len();
            self.evaluation_state.push(Cell::new(0.0));
            self.index.insert(key.to_string(), idx);
            Proxy::resolved(idx)
        }).collect();

        let requested = needs.iter().map(|&key| {
            let proxy = Proxy::unresolved();
            self.pending_requests.push((key.to_string(), proxy.clone()));
            proxy
        }).collect();

        (offered, requested)
    }

    /// Roda uma única vez, depois que todo mundo já se inscreveu. Resolve
    /// cada input pendente contra a posição já conhecida (de quem ofereceu
    /// aquele nome). Se algum input não tiver provedor, é erro — o resto
    /// pode ter ficado parcialmente resolvido, então não adianta continuar
    /// rodando a simulação depois disso falhar.
    pub fn resolve(&mut self) -> Result<(), String> {
        for (key, proxy) in &self.pending_requests {
            match self.index.get(key) {
                Some(&idx) => proxy.index.set(idx),
                None => return Err(format!(
                    "input '{key}' declarado em subscribe() mas nenhum componente oferece esse slot"
                )),
            }
        }
        Ok(())
    }
}
