/* tep/units/mod.rs */

/** Os 5 blocos químicos do TEP: Feed/Reactor/Separator/Stripper/Compressor, todos `#[monjolo::
tasks]` (issue 10) — vários métodos nomeados por unidade, cada um seu próprio `needs`/`offers`.
`flows.rs`/`heat.rs`/`derivatives.rs`/`measured.rs` (toda a álgebra transversal do FORTRAN
original — vazões entre unidades, cargas térmicas, o balanço de massa/energia final, conversões
pra XMEAS) foram totalmente dissolvidos: cada pedaço absorvido pela unidade que o produz. Os 3
analisadores de composição também (`reactor_feed_analyzer`→Compressor, `purge_analyzer`→
Separator, `product_analyzer`→Stripper). O único pedaço de `measured.rs` sem dono natural
(detecção de shutdown, agrega 3 unidades) virou `diagnostics::shutdown_detector`.

Ordem de avaliação da fase (A): desde a extensão de `component::sort_phase_a` (issue 10), a ordem
não é mais uma cadeia `after=[...]` só — é inferida automaticamente casando `needs`↔`offers` entre
TODOS os nós (struct inteira ou tarefa de método), com `after` como desempate. Nenhuma das 5
unidades declara `after` hoje — a ordem inteira (Reactor→Separator/Compressor→Stripper e as
dependências cruzadas de cada tarefa) sai só do casamento de chave.
*/
pub mod compressor;
pub mod feed;
pub mod reactor;
pub mod separator;
pub mod stripper;

/** Só pra testes: roda tarefas (`#[task]`) isoladas, sem a planta inteira. Constrói a unidade dona
(`unit`), oferece "de fora" os sinais que as tarefas precisam e que nenhuma outra tarefa construída
aqui oferece (`seed`), constrói só as tarefas pedidas (`task`, pelo nome `Unidade::metodo`) e lê
qualquer chave depois (`read`) — tudo por chave, do mesmo jeito que a planta de verdade conversa.
*/
#[cfg(test)]
pub(crate) mod harness {
    use monjolo::dynamic_model::DynamicModel;
    use monjolo::snapshot::Snapshot;
    use monjolo::state_registry::StateRegistry;
    use std::cell::RefCell;
    use std::rc::Rc;

    pub struct Harness {
        registry: Rc<RefCell<StateRegistry>>,
    }

    impl Harness {
        pub fn new() -> Self {
            Self { registry: StateRegistry::shared() }
        }

        pub fn unit<T>(&self, build: impl FnOnce(&mut StateRegistry) -> Rc<T>) -> Rc<T> {
            build(&mut self.registry.borrow_mut())
        }

        pub fn seed(&self, key: &str, value: f64) {
            let (offered, _) = self.registry.borrow_mut().subscribe(&[key], &[]);
            offered[0].set(value);
        }

        /* Semeia uma mistura (`prefix.a`, `prefix.b`, ...) — mesma convenção de nome que o `::<Fase>` da macro. */
        pub fn seed_mixture(&self, prefix: &str, values: &[f64]) {
            for (i, value) in values.iter().enumerate() {
                self.seed(&format!("{prefix}.{}", (b'a' + i as u8) as char), *value);
            }
        }

        pub fn task(&self, name: &str) -> Box<dyn DynamicModel> {
            let descriptor = monjolo::inventory::iter::<monjolo::ComponentDescriptor>()
                .find(|d| d.name == name)
                .unwrap_or_else(|| panic!("tarefa `{name}` não está registrada no inventory"));
            (descriptor.construct)(&mut self.registry.borrow_mut(), &Snapshot::from_pairs(&[]))
                .unwrap_or_else(|| panic!("tarefa `{name}` não devolveu componente"))
        }

        pub fn resolve(&self) {
            self.registry.borrow_mut().resolve().expect("todo need deveria ter um offer (tarefa ou seed)");
        }

        pub fn read(&self, key: &str) -> f64 {
            let (_, needed) = self.registry.borrow_mut().subscribe(&[], &[key]);
            self.resolve();
            needed[0].get()
        }

        pub fn read_mixture(&self, prefix: &str, len: usize) -> Vec<f64> {
            (0..len).map(|i| self.read(&format!("{prefix}.{}", (b'a' + i as u8) as char))).collect()
        }
    }
}
