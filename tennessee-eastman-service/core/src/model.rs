// core/model.rs
//
// Interface central do sistema de modelagem dinâmica.
// Todos os componentes (planta, atuador, sensor) implementam DynamicModel.

pub trait DynamicModel {
    fn state_size(&self) -> usize;
    fn dynamics(&mut self, state: &[f64]) -> Vec<f64>;
    fn name(&self) -> &'static str {
        "unnamed"
    }
}
