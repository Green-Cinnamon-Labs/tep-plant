/* tep/measurements.rs */

/** Mesma situação de Flows/Heat: sem estado próprio, precisa de Reactor + Separator + Stripper +
Compressor + Flows + Heat ao mesmo tempo (é o mais "a jusante" dos sete — Block 35 do teprob.f,
preenche xmeas[0..22] e decide shutdown). Física original em _deprecated_2.rs, não portada ainda
pelo mesmo motivo dos outros dois.
*/

use monjolo::dynamic_model::DynamicModel;

pub struct Measurements;

impl Measurements {
    pub fn new() -> Self {
        Self
    }
}

impl DynamicModel for Measurements {
    fn name(&self) -> &'static str {
        "Measurements"
    }

    fn evaluate(&self) {
        todo!("precisa de Reactor+Separator+Stripper+Compressor+Flows+Heat ao mesmo tempo")
    }
}
