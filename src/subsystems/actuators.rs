// tep/subsystems/actuators.rs
//
// Valve/Agitator — movidos de dentro do `monjolo` pra cá (2026-07-30):
// são componentes físicos específicos do TEP, não blocos genéricos do
// framework. Moram em subsystems/ junto com Reactor/Separator/Stripper/
// Compressor porque são exatamente a mesma coisa — componentes com estado
// e derivada próprios, add_dynamic'd no composto (Art. 2.6). Arquivo no
// plural (`actuators`, não `actuator`) porque guarda mais de um tipo, ao
// contrário dos outros subsistemas (um tipo por arquivo).
//
// Cada um é as duas coisas ao mesmo tempo — `DynamicModel`
// (participa de evaluate()/RK4, dinâmica de 1ª ordem real) *e*
// `monjolo::actuator::Actuator` (`impl Actuator for Valve`/`Agitator`,
// aceita comando de fora via `write()`) — não há conflito, são dois traits
// distintos sobre o mesmo objeto.
//
// `command` é `Cell<f64>`, não um campo simples: `write(&self, ...)`
// (contrato de `Actuator`) e `evaluate(&self)` (contrato de `DynamicModel`)
// não recebem `&mut self`, mas precisam mutar `command` — mutabilidade
// interior, mesmo raciocínio de `EvaluationState`/`Proxy`.
//
// Cada instância de `Valve` precisa de um `name` único (ex.: "feed_a",
// "cooling_water") pra não colidir no StateRegistry. `Agitator` é único na
// planta, sem parâmetro de nome.

use std::cell::Cell;

use monjolo::actuator::Actuator;
use monjolo::dynamic_model::DynamicModel;
use monjolo::state_registry::{Proxy, StateRegistry};

// ── Valve ───────────────────────────────────────────────────────────
// Models a control valve with first-order lag: d(position)/dt = (command - position) / tau
// State: [position]  (one variable: current valve position, 0–100 %)

pub struct Valve {
    tau: f64,
    command: Cell<f64>,
    position: Proxy,
    derivative: Proxy,
}

impl Valve {
    pub fn new(registry: &mut StateRegistry, name: &str, tau: f64) -> Self {
        let position_key = format!("valve.{name}.position");
        let derivative_key = format!("valve.{name}.position.derivative");
        let (offered, _) = registry.subscribe(&[&position_key, &derivative_key], &[]);

        Self {
            tau,
            command: Cell::new(0.0),
            position: offered[0].clone(),
            derivative: offered[1].clone(),
        }
    }
}

impl Actuator for Valve {
    fn write(&self, value: f64) {
        self.command.set(value);
    }
}

impl DynamicModel for Valve {
    fn name(&self) -> &'static str {
        "Valve"
    }

    fn evaluate(&self) {
        let position = self.position.get();
        self.derivative.set((self.command.get() - position) / self.tau);
    }
}

// ── Agitator ──────────────────────────────────────────────────────────────────
// Reactor agitator speed — same first-order dynamics as a valve but
// controls mixing intensity (agsp), not fluid flow.
// State: [speed]  (one variable: current agitator speed, 0–100 %)

pub struct Agitator {
    tau: f64,
    command: Cell<f64>,
    speed: Proxy,
    derivative: Proxy,
}

impl Agitator {
    pub fn new(registry: &mut StateRegistry, tau: f64) -> Self {
        let (offered, _) =
            registry.subscribe(&["agitator.speed", "agitator.speed.derivative"], &[]);

        Self {
            tau,
            command: Cell::new(0.0),
            speed: offered[0].clone(),
            derivative: offered[1].clone(),
        }
    }
}

impl Actuator for Agitator {
    fn write(&self, value: f64) {
        self.command.set(value);
    }
}

impl DynamicModel for Agitator {
    fn name(&self) -> &'static str {
        "Agitator"
    }

    fn evaluate(&self) {
        let speed = self.speed.get();
        self.derivative.set((self.command.get() - speed) / self.tau);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valve_derivative_is_real_not_a_stub() {
        let registry = StateRegistry::shared();
        let valve = Valve::new(&mut registry.borrow_mut(), "feed_a", 2.0);
        registry.borrow_mut().resolve().unwrap();

        valve.write(50.0);
        // posição nasce em 0.0 (default do slot) — derivada esperada: (50-0)/2 = 25
        valve.evaluate();
        assert_eq!(valve.derivative.get(), 25.0);
    }
}
