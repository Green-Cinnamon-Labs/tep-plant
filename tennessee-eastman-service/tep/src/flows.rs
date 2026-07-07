// tep/flows.rs
//
// Flows não tem estado próprio nenhum — diferente de Reactor/Separator/
// Stripper/Compressor (que têm um slice de estado integrado), Flows é pura
// agregação algébrica dos quatro ao mesmo tempo (mais posições de válvula e
// flags de distúrbio). Isso levanta uma dúvida em aberto que ainda não
// resolvemos: faz sentido Flows/Heat/Measurements serem DynamicModel de
// verdade (a interface pressupõe um `state: &[f64]` próprio que eles não
// têm), ou deveriam ser outra coisa — um consumidor puro, no espírito da
// seção 3 do plano (Outside/AcquisitionLayer)? Por enquanto ficam com o
// contrato mesmo assim, com `evaluate()` em todo!().
//
// A física original (Blocks 19-31 do teprob.f) está em _deprecated_2.rs. Não
// portei ela pra cá ainda porque os tipos que ela consumia (ReactorOut,
// SeparatorOut, StripperOut, CompressorOut) não existem mais — Reactor/
// Separator/Stripper/Compressor agora devolvem Vec<StateSlot>, não structs
// tipados — e ValvePositions também não existe mais. Portar exige decidir
// primeiro como os inputs de múltiplos componentes chegam aqui (Proxy vs.
// parâmetros nomeados) antes de reescrever a lógica.

use simulation_framework::dynamic_model::DynamicModel;
use simulation_framework::state_registry::EvaluationState;

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

    fn evaluate(&self, _state: &[f64], _eval: &EvaluationState) {
        todo!("precisa de Reactor+Separator+Stripper+Compressor+válvulas ao mesmo tempo — ver comentário no topo do arquivo")
    }
}
