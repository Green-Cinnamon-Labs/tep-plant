// tep/lib.rs
//
// Só o que é específico do TEP mora aqui. Framework de simulação genérico
// (DynamicModel, StateRegistry, Integrator, atuador/sensor de 1ª ordem,
// distúrbio cúbico) mora em simulation-framework (crate irmão).
pub mod constants;
pub mod disturbance_state;
pub mod initial_state;
pub mod model;
pub mod subsystems;
pub mod thermo;