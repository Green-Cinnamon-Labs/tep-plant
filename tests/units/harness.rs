/** Roda tarefas (`#[task]`) isoladas, sem a planta inteira. Constrói a unidade dona (`unit`),
oferece "de fora" os sinais que as tarefas precisam e que nenhuma outra tarefa construída aqui
oferece (`seed`), constrói só as tarefas pedidas (`task`, pelo nome `Unidade::metodo`) e lê qualquer
chave depois (`read`) — tudo por chave, do mesmo jeito que a planta de verdade conversa.
*/
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
