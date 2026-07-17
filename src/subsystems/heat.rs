// tep/heat.rs
//
// Mesma situação de Flows (ver flows.rs): sem estado próprio, precisa de
// Reactor + Stripper + Flows ao mesmo tempo. Física original (Blocks 32-34)
// em _deprecated_2.rs, não portada ainda pelo mesmo motivo (tipos de entrada
// não existem mais nesse formato).

use monjolo::dynamic_model::DynamicModel;

pub struct Heat;

impl Heat {
    pub fn new() -> Self {
        Self
    }
}

impl DynamicModel for Heat {
    fn name(&self) -> &'static str {
        "Heat"
    }

    fn evaluate(&self) {
        todo!("precisa de Reactor+Stripper+Flows ao mesmo tempo — ver comentário em flows.rs")
    }
}
