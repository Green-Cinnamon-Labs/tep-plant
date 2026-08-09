/* tep/subsystems/actuators.rs */

/** Os 12 atuadores físicos da planta (XMV-1 a XMV-12) não têm tipo Rust próprio — ver
`monjolo::actuator::model::Actuator` (Art. 12.8 do CONTRIBUTING): cada um é só uma chave do
StateRegistry + uma lei física (closure), passadas pra `Actuator::new()`. Este arquivo só agrupa
essas 12 construções em funções nomeadas, uma por atuador, pra `model.rs::build_tep()` ficar limpo
— cada função aqui é a única fonte da verdade pra chave/τ daquele atuador. τ vem de VTAU(n) em
teprob.f, cross-checado contra as equações físicas que consomem VPOS(I) (`docs/_deprecated_1.rs`
rotula XMV-9/XMV-11 trocados — ver histórico deste arquivo).
*/

use monjolo::actuator::model::Actuator;
use monjolo::state_registry::StateRegistry;

/* XMV-1: D Feed Flow. VTAU(1) = 8s. */
pub fn feed_d(registry: &mut StateRegistry) -> Actuator {
    Actuator::new(registry, "valve.feed_d.position", |command, position| {
        let tau = 8.0 / 3600.0;
        (command - position) / tau
    })
}

/* XMV-2: E Feed Flow. VTAU(2) = 8s. */
pub fn feed_e(registry: &mut StateRegistry) -> Actuator {
    Actuator::new(registry, "valve.feed_e.position", |command, position| {
        let tau = 8.0 / 3600.0;
        (command - position) / tau
    })
}

/* XMV-3: A Feed Flow. VTAU(3) = 6s. */
pub fn feed_a(registry: &mut StateRegistry) -> Actuator {
    Actuator::new(registry, "valve.feed_a.position", |command, position| {
        let tau = 6.0 / 3600.0;
        (command - position) / tau
    })
}

/* XMV-4: A&C Feed Flow (alimentação combinada). VTAU(4) = 9s. */
pub fn feed_ac(registry: &mut StateRegistry) -> Actuator {
    Actuator::new(registry, "valve.feed_ac.position", |command, position| {
        let tau = 9.0 / 3600.0;
        (command - position) / tau
    })
}

/* XMV-5: Compressor Recycle Valve. VTAU(5) = 7s. */
pub fn compressor_recycle(registry: &mut StateRegistry) -> Actuator {
    Actuator::new(registry, "valve.compressor_recycle.position", |command, position| {
        let tau = 7.0 / 3600.0;
        (command - position) / tau
    })
}

/* XMV-6: Purge Valve. VTAU(6) = 5s. */
pub fn purge(registry: &mut StateRegistry) -> Actuator {
    Actuator::new(registry, "valve.purge.position", |command, position| {
        let tau = 5.0 / 3600.0;
        (command - position) / tau
    })
}

/* XMV-7: Separator Pot Liquid Flow (underflow do separador). VTAU(7) = 5s. */
pub fn separator_underflow(registry: &mut StateRegistry) -> Actuator {
    Actuator::new(registry, "valve.separator_underflow.position", |command, position| {
        let tau = 5.0 / 3600.0;
        (command - position) / tau
    })
}

/* XMV-8: Stripper Liquid Product Flow. VTAU(8) = 5s. */
pub fn stripper_product(registry: &mut StateRegistry) -> Actuator {
    Actuator::new(registry, "valve.stripper_product.position", |command, position| {
        let tau = 5.0 / 3600.0;
        (command - position) / tau
    })
}

/* XMV-9: Stripper Steam Valve. VTAU(9) = 120s — a mais lenta das 12 (não a de resfriamento do
condensador, como docs/_deprecated_1.rs sugere).
*/
pub fn stripper_steam(registry: &mut StateRegistry) -> Actuator {
    Actuator::new(registry, "valve.stripper_steam.position", |command, position| {
        let tau = 120.0 / 3600.0;
        (command - position) / tau
    })
}

/* XMV-10: Reactor Cooling Water Flow. VTAU(10) = 5s. */
pub fn reactor_cooling_water(registry: &mut StateRegistry) -> Actuator {
    Actuator::new(registry, "valve.reactor_cooling_water.position", |command, position| {
        let tau = 5.0 / 3600.0;
        (command - position) / tau
    })
}

/* XMV-11: Condenser Cooling Water Flow. VTAU(11) = 5s — igual à maioria das demais (não 120s, como
docs/_deprecated_1.rs sugere).
*/
pub fn condenser_cooling_water(registry: &mut StateRegistry) -> Actuator {
    Actuator::new(registry, "valve.condenser_cooling_water.position", |command, position| {
        let tau = 5.0 / 3600.0;
        (command - position) / tau
    })
}

/* XMV-12: Agitator Speed. VTAU(12) = 5s. Único na planta, sem chave por nome. */
pub fn agitator(registry: &mut StateRegistry) -> Actuator {
    Actuator::new(registry, "agitator.speed", |command, speed| {
        let tau = 5.0 / 3600.0;
        (command - speed) / tau
    })
}
