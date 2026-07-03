// core/model.rs
//
// Interface central do sistema de modelagem dinâmica.
// Todos os componentes (planta, atuador, sensor) implementam DynamicModel.

pub trait DynamicModel {
    fn state_size(&self) -> usize;
    fn dynamics(&mut self, state: &[f64]) -> Vec<f64>;
    fn name(&self) -> &'static str { "unnamed" }

    // Extensões com default no-op — sobrescritas pelo modelo concreto quando necessário.
    fn set_commands(&mut self, _xmv: &[f64]) {}
    fn set_disturbances(&mut self, _dv: &[f64]) {}
    fn advance_time(&mut self, _dt: f64) {}
    fn set_disturbance_step_magnitude(&mut self, _idx: usize, _mag: f64) {}
    fn measurements(&self) -> Vec<f64> { vec![] }
}
