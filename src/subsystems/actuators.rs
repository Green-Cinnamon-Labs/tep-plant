/* tep/subsystems/actuators.rs */

/** Um tipo concreto por atuador físico do TEP (11 válvulas, XMV-1 a XMV-11, mais o agitador,
XMV-12) — não um `Valve` genérico parametrizado por nome/τ. `monjolo::actuator::Actuator` só
define o conceito (`fn write(&self, value: f64)`); quem dá corpo a cada atuador é este arquivo, um
`struct` + `impl Actuator` + `impl DynamicModel` por tipo, cada um com sua própria chave de
StateRegistry e constante τ embutidas — não há parâmetro de nome/τ no construtor, o tipo já diz
quem ele é. Arquivo no plural (`actuators`, não `actuator`) porque guarda vários tipos, ao
contrário dos outros subsistemas (um tipo por arquivo).

Cada um é as duas coisas ao mesmo tempo — `DynamicModel` (participa de evaluate()/RK4, dinâmica de
1ª ordem real: d(posição)/dt = (comando - posição) / τ) *e* `Actuator` (aceita comando de fora via
`write()`) — não há conflito, são dois traits distintos sobre o mesmo objeto.

`command` é `Cell<f64>`, não um campo simples: `write(&self, ...)` (contrato de `Actuator`) e
`evaluate(&self)` (contrato de `DynamicModel`) não recebem `&mut self`, mas precisam mutar
`command` — mutabilidade interior, mesmo raciocínio de `EvaluationState`/`Proxy`.

τ de cada válvula vem de `VTAU(1..11)` em `teprob.f` (segundos, convertidos pra horas). Ordem
canônica do cabeçalho de `teprob.f` (linhas 105-116, anotado "[Corrected Order]" pra XMV-1..3),
cross-checada contra as equações físicas que consomem `VPOS(I)` (ex.: `FWR =
VPOS(10)*VRNG(10)/100` prova XMV-10 = água de resfriamento do reator) — `docs/_deprecated_1.rs`
rotula essa mesma ordem errado (troca XMV-9 com XMV-11).
*/

use std::cell::Cell;

use monjolo::actuator::Actuator;
use monjolo::dynamic_model::DynamicModel;
use monjolo::state_registry::{Proxy, StateRegistry};

/* ── FeedDValve (XMV-1) ── */

/** D Feed Flow. τ = `VTAU(1)` em `teprob.f`, 8 segundos.
*/
pub struct FeedDValve {
    command: Cell<f64>,
    position: Proxy,
    derivative: Proxy,
}

impl FeedDValve {
    const TAU_HOURS: f64 = 8.0 / 3600.0;

    pub fn new(registry: &mut StateRegistry) -> Self {
        let (offered, _) = registry
            .subscribe(&["valve.feed_d.position", "valve.feed_d.position.derivative"], &[]);

        Self {
            command: Cell::new(0.0),
            position: offered[0].clone(),
            derivative: offered[1].clone(),
        }
    }
}

impl Actuator for FeedDValve {
    fn write(&self, value: f64) {
        self.command.set(value);
    }
}

impl DynamicModel for FeedDValve {
    fn name(&self) -> &'static str {
        "FeedDValve"
    }

    fn evaluate(&self) {
        let position = self.position.get();
        self.derivative.set((self.command.get() - position) / Self::TAU_HOURS);
    }
}

/* ── FeedEValve (XMV-2) ── */

/** E Feed Flow. τ = `VTAU(2)` em `teprob.f`, 8 segundos.
*/
pub struct FeedEValve {
    command: Cell<f64>,
    position: Proxy,
    derivative: Proxy,
}

impl FeedEValve {
    const TAU_HOURS: f64 = 8.0 / 3600.0;

    pub fn new(registry: &mut StateRegistry) -> Self {
        let (offered, _) = registry
            .subscribe(&["valve.feed_e.position", "valve.feed_e.position.derivative"], &[]);

        Self {
            command: Cell::new(0.0),
            position: offered[0].clone(),
            derivative: offered[1].clone(),
        }
    }
}

impl Actuator for FeedEValve {
    fn write(&self, value: f64) {
        self.command.set(value);
    }
}

impl DynamicModel for FeedEValve {
    fn name(&self) -> &'static str {
        "FeedEValve"
    }

    fn evaluate(&self) {
        let position = self.position.get();
        self.derivative.set((self.command.get() - position) / Self::TAU_HOURS);
    }
}

/* ── FeedAValve (XMV-3) ── */

/** A Feed Flow. τ = `VTAU(3)` em `teprob.f`, 6 segundos.
*/
pub struct FeedAValve {
    command: Cell<f64>,
    position: Proxy,
    derivative: Proxy,
}

impl FeedAValve {
    const TAU_HOURS: f64 = 6.0 / 3600.0;

    pub fn new(registry: &mut StateRegistry) -> Self {
        let (offered, _) = registry
            .subscribe(&["valve.feed_a.position", "valve.feed_a.position.derivative"], &[]);

        Self {
            command: Cell::new(0.0),
            position: offered[0].clone(),
            derivative: offered[1].clone(),
        }
    }
}

impl Actuator for FeedAValve {
    fn write(&self, value: f64) {
        self.command.set(value);
    }
}

impl DynamicModel for FeedAValve {
    fn name(&self) -> &'static str {
        "FeedAValve"
    }

    fn evaluate(&self) {
        let position = self.position.get();
        self.derivative.set((self.command.get() - position) / Self::TAU_HOURS);
    }
}

/* ── FeedACValve (XMV-4) ── */

/** A&C Feed Flow (alimentação combinada). τ = `VTAU(4)` em `teprob.f`, 9 segundos.
*/
pub struct FeedACValve {
    command: Cell<f64>,
    position: Proxy,
    derivative: Proxy,
}

impl FeedACValve {
    const TAU_HOURS: f64 = 9.0 / 3600.0;

    pub fn new(registry: &mut StateRegistry) -> Self {
        let (offered, _) = registry
            .subscribe(&["valve.feed_ac.position", "valve.feed_ac.position.derivative"], &[]);

        Self {
            command: Cell::new(0.0),
            position: offered[0].clone(),
            derivative: offered[1].clone(),
        }
    }
}

impl Actuator for FeedACValve {
    fn write(&self, value: f64) {
        self.command.set(value);
    }
}

impl DynamicModel for FeedACValve {
    fn name(&self) -> &'static str {
        "FeedACValve"
    }

    fn evaluate(&self) {
        let position = self.position.get();
        self.derivative.set((self.command.get() - position) / Self::TAU_HOURS);
    }
}

/* ── CompressorRecycleValve (XMV-5) ── */

/** Compressor Recycle Valve. τ = `VTAU(5)` em `teprob.f`, 7 segundos.
*/
pub struct CompressorRecycleValve {
    command: Cell<f64>,
    position: Proxy,
    derivative: Proxy,
}

impl CompressorRecycleValve {
    const TAU_HOURS: f64 = 7.0 / 3600.0;

    pub fn new(registry: &mut StateRegistry) -> Self {
        let (offered, _) = registry.subscribe(
            &["valve.compressor_recycle.position", "valve.compressor_recycle.position.derivative"],
            &[],
        );

        Self {
            command: Cell::new(0.0),
            position: offered[0].clone(),
            derivative: offered[1].clone(),
        }
    }
}

impl Actuator for CompressorRecycleValve {
    fn write(&self, value: f64) {
        self.command.set(value);
    }
}

impl DynamicModel for CompressorRecycleValve {
    fn name(&self) -> &'static str {
        "CompressorRecycleValve"
    }

    fn evaluate(&self) {
        let position = self.position.get();
        self.derivative.set((self.command.get() - position) / Self::TAU_HOURS);
    }
}

/* ── PurgeValve (XMV-6) ── */

/** Purge Valve. τ = `VTAU(6)` em `teprob.f`, 5 segundos.
*/
pub struct PurgeValve {
    command: Cell<f64>,
    position: Proxy,
    derivative: Proxy,
}

impl PurgeValve {
    const TAU_HOURS: f64 = 5.0 / 3600.0;

    pub fn new(registry: &mut StateRegistry) -> Self {
        let (offered, _) = registry
            .subscribe(&["valve.purge.position", "valve.purge.position.derivative"], &[]);

        Self {
            command: Cell::new(0.0),
            position: offered[0].clone(),
            derivative: offered[1].clone(),
        }
    }
}

impl Actuator for PurgeValve {
    fn write(&self, value: f64) {
        self.command.set(value);
    }
}

impl DynamicModel for PurgeValve {
    fn name(&self) -> &'static str {
        "PurgeValve"
    }

    fn evaluate(&self) {
        let position = self.position.get();
        self.derivative.set((self.command.get() - position) / Self::TAU_HOURS);
    }
}

/* ── SeparatorUnderflowValve (XMV-7) ── */

/** Separator Pot Liquid Flow (underflow do separador). τ = `VTAU(7)` em `teprob.f`, 5 segundos.
*/
pub struct SeparatorUnderflowValve {
    command: Cell<f64>,
    position: Proxy,
    derivative: Proxy,
}

impl SeparatorUnderflowValve {
    const TAU_HOURS: f64 = 5.0 / 3600.0;

    pub fn new(registry: &mut StateRegistry) -> Self {
        let (offered, _) = registry.subscribe(
            &["valve.separator_underflow.position", "valve.separator_underflow.position.derivative"],
            &[],
        );

        Self {
            command: Cell::new(0.0),
            position: offered[0].clone(),
            derivative: offered[1].clone(),
        }
    }
}

impl Actuator for SeparatorUnderflowValve {
    fn write(&self, value: f64) {
        self.command.set(value);
    }
}

impl DynamicModel for SeparatorUnderflowValve {
    fn name(&self) -> &'static str {
        "SeparatorUnderflowValve"
    }

    fn evaluate(&self) {
        let position = self.position.get();
        self.derivative.set((self.command.get() - position) / Self::TAU_HOURS);
    }
}

/* ── StripperProductValve (XMV-8) ── */

/** Stripper Liquid Product Flow. τ = `VTAU(8)` em `teprob.f`, 5 segundos.
*/
pub struct StripperProductValve {
    command: Cell<f64>,
    position: Proxy,
    derivative: Proxy,
}

impl StripperProductValve {
    const TAU_HOURS: f64 = 5.0 / 3600.0;

    pub fn new(registry: &mut StateRegistry) -> Self {
        let (offered, _) = registry.subscribe(
            &["valve.stripper_product.position", "valve.stripper_product.position.derivative"],
            &[],
        );

        Self {
            command: Cell::new(0.0),
            position: offered[0].clone(),
            derivative: offered[1].clone(),
        }
    }
}

impl Actuator for StripperProductValve {
    fn write(&self, value: f64) {
        self.command.set(value);
    }
}

impl DynamicModel for StripperProductValve {
    fn name(&self) -> &'static str {
        "StripperProductValve"
    }

    fn evaluate(&self) {
        let position = self.position.get();
        self.derivative.set((self.command.get() - position) / Self::TAU_HOURS);
    }
}

/* ── StripperSteamValve (XMV-9) ── */

/** Stripper Steam Valve. τ = `VTAU(9)` em `teprob.f`, 120 segundos — a mais lenta das 12 (não a
de resfriamento do condensador, como `docs/_deprecated_1.rs` sugere).
*/
pub struct StripperSteamValve {
    command: Cell<f64>,
    position: Proxy,
    derivative: Proxy,
}

impl StripperSteamValve {
    const TAU_HOURS: f64 = 120.0 / 3600.0;

    pub fn new(registry: &mut StateRegistry) -> Self {
        let (offered, _) = registry.subscribe(
            &["valve.stripper_steam.position", "valve.stripper_steam.position.derivative"],
            &[],
        );

        Self {
            command: Cell::new(0.0),
            position: offered[0].clone(),
            derivative: offered[1].clone(),
        }
    }
}

impl Actuator for StripperSteamValve {
    fn write(&self, value: f64) {
        self.command.set(value);
    }
}

impl DynamicModel for StripperSteamValve {
    fn name(&self) -> &'static str {
        "StripperSteamValve"
    }

    fn evaluate(&self) {
        let position = self.position.get();
        self.derivative.set((self.command.get() - position) / Self::TAU_HOURS);
    }
}

/* ── ReactorCoolingWaterValve (XMV-10) ── */

/** Reactor Cooling Water Flow. τ = `VTAU(10)` em `teprob.f`, 5 segundos.
*/
pub struct ReactorCoolingWaterValve {
    command: Cell<f64>,
    position: Proxy,
    derivative: Proxy,
}

impl ReactorCoolingWaterValve {
    const TAU_HOURS: f64 = 5.0 / 3600.0;

    pub fn new(registry: &mut StateRegistry) -> Self {
        let (offered, _) = registry.subscribe(
            &[
                "valve.reactor_cooling_water.position",
                "valve.reactor_cooling_water.position.derivative",
            ],
            &[],
        );

        Self {
            command: Cell::new(0.0),
            position: offered[0].clone(),
            derivative: offered[1].clone(),
        }
    }
}

impl Actuator for ReactorCoolingWaterValve {
    fn write(&self, value: f64) {
        self.command.set(value);
    }
}

impl DynamicModel for ReactorCoolingWaterValve {
    fn name(&self) -> &'static str {
        "ReactorCoolingWaterValve"
    }

    fn evaluate(&self) {
        let position = self.position.get();
        self.derivative.set((self.command.get() - position) / Self::TAU_HOURS);
    }
}

/* ── CondenserCoolingWaterValve (XMV-11) ── */

/** Condenser Cooling Water Flow. τ = `VTAU(11)` em `teprob.f`, 5 segundos — igual à maioria das
demais (não 120s, como `docs/_deprecated_1.rs` sugere).
*/
pub struct CondenserCoolingWaterValve {
    command: Cell<f64>,
    position: Proxy,
    derivative: Proxy,
}

impl CondenserCoolingWaterValve {
    const TAU_HOURS: f64 = 5.0 / 3600.0;

    pub fn new(registry: &mut StateRegistry) -> Self {
        let (offered, _) = registry.subscribe(
            &[
                "valve.condenser_cooling_water.position",
                "valve.condenser_cooling_water.position.derivative",
            ],
            &[],
        );

        Self {
            command: Cell::new(0.0),
            position: offered[0].clone(),
            derivative: offered[1].clone(),
        }
    }
}

impl Actuator for CondenserCoolingWaterValve {
    fn write(&self, value: f64) {
        self.command.set(value);
    }
}

impl DynamicModel for CondenserCoolingWaterValve {
    fn name(&self) -> &'static str {
        "CondenserCoolingWaterValve"
    }

    fn evaluate(&self) {
        let position = self.position.get();
        self.derivative.set((self.command.get() - position) / Self::TAU_HOURS);
    }
}

/* ── Agitator (XMV-12) ── */

/** Reactor agitator speed — same first-order dynamics as a valve but controls mixing intensity
(agsp), not fluid flow. τ = `VTAU(12)` em `teprob.f`, 5 segundos. Único na planta, sem chave por
nome.
*/
pub struct Agitator {
    command: Cell<f64>,
    speed: Proxy,
    derivative: Proxy,
}

impl Agitator {
    
    const TAU_HOURS: f64 = 5.0 / 3600.0;

    pub fn new(registry: &mut StateRegistry) -> Self {
        let (offered, _) =
            registry.subscribe(&["agitator.speed", "agitator.speed.derivative"], &[]);

        Self {
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
        self.derivative.set((self.command.get() - speed) / Self::TAU_HOURS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valve_derivative_is_real_not_a_stub() {
        let registry = StateRegistry::shared();
        let valve = FeedAValve::new(&mut registry.borrow_mut());
        registry.borrow_mut().resolve().unwrap();

        valve.write(50.0);
        /* posição nasce em 0.0 (default do slot) — derivada esperada: (50-0)/τ */
        valve.evaluate();
        assert_eq!(valve.derivative.get(), 50.0 / FeedAValve::TAU_HOURS);
    }
}
