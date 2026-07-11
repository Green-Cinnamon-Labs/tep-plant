// tep/flows.rs
//
// Flows não tem estado próprio nenhum — diferente de Reactor/Separator/
// Stripper/Compressor (que têm um slice de estado integrado). É pura
// agregação algébrica dos quatro ao mesmo tempo (mais posições de válvula e
// flags de distúrbio). A dúvida que existia aqui (evaluate() pressupunha um
// `state: &[f64]` que Flows não tem) está resolvida: DynamicModel não
// significa "produz derivada", significa "tem alguma dinâmica a avaliar" —
// mesmo sem estado próprio, Flows é afetado pela integração porque os
// valores que ele lê (via Proxy) mudam a cada rodada. `evaluate(&self)` não
// tem parâmetro nenhum pra ninguém, com ou sem estado próprio.
//
// A física original (Blocks 19-31 do teprob.f) está em _deprecated_2.rs.
// Ainda não portada — falta só decidir os `needs` (bem mais numerosos que os
// outros: os quatro subsistemas + posições de válvula + flags de distúrbio)
// e escrever a lógica com eles.

use monjolo::dynamic_model::DynamicModel;

pub struct Flows;

impl Flows {
    pub fn new() -> Self {
        Self
    }
}

impl DynamicModel for Flows {
    fn name(&self) -> &'static str {
        "Flows"
    }

    fn evaluate(&self) {
        todo!("precisa de Reactor+Separator+Stripper+Compressor+válvulas ao mesmo tempo — ver comentário no topo do arquivo")
    }
}