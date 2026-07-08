// tep/lib.rs
//
// Só o que é específico do TEP mora aqui. Framework de simulação genérico
// (DynamicModel, StateRegistry, Integrator, atuador/sensor de 1ª ordem,
// distúrbio cúbico) mora em simulation-framework (crate irmão).
pub mod compressor;
pub mod constants;
pub mod disturbance;
pub mod disturbance_state;
pub mod flows;
pub mod heat;
pub mod initial_state;
pub mod measurements;
pub mod model;
pub mod reactor;
pub mod separator;
pub mod stripper;
pub mod thermo;

// subsystems.rs original (física de todos os 7 acima) preservada em
// _deprecated_2.rs — não é módulo do crate, só referência.