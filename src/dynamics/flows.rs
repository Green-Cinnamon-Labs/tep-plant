/* tep/dynamics/flows.rs */

use crate::physics::constants::TepConstants;

/** Só sobra o Block 23 (slot 7, reator→separador) e o Block 22's `agitation_factor` — o resto já
migrou pras unidades donas (issue 10: Feed, Compressor, Separator, Stripper). Ambos aguardam a
migração do Reactor (último passo) — `flows.stream_flow.7` é consumido pelo balanço do Reator
(ainda em `derivatives.rs`) e pelo `heat.rs` (Block 32, ainda não migrado); `agitation_factor`
também é só consumido por `heat.rs`. Sem `after` declarado — o sort automático por `needs`/
`offers` já ordena Flows depois de Reactor/Separator (únicos donos reais de quem Flows precisa).
*/
#[monjolo::dynamic_model]
pub struct Flows {
    #[need(key = "reactor.pressure")]
    reactor_pressure: f64,
    #[need(prefix = "reactor.vapor_composition", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    reactor_vapor_composition: [f64; 8],
    #[need(key = "separator.pressure")]
    separator_pressure: f64,
    #[need(key = "agitator.speed")]
    agitator_speed: f64,

    #[offer(key = "flows.stream_flow.7")]
    reactor_to_separator_flow: f64,
    #[offer(key = "flows.agitation_factor")]
    agitation_factor: f64,

    constants: TepConstants,
}

impl Flows {
    fn compute(&self) {
        let reactor_vapor = self.reactor_vapor_composition();
        let reactor_pressure = self.reactor_pressure();
        let separator_pressure = self.separator_pressure();

        /* Block 23: fluxo reator→separador, dependente de ΔP (sem válvula). */
        let mol_weight: f64 = (0..8).map(|i| reactor_vapor[i] * self.constants.xmw[i]).sum();
        let reactor_to_separator_flow =
            4574.21 * (reactor_pressure - separator_pressure).max(0.0).sqrt() * (1.0 - 0.25 * 0.0) / mol_weight; /* disturbance channel 11, neutro */

        /* Block 22: AGSP (velocidade do agitador) — sem válvula, atuador direto. */
        let agitation_factor = (self.agitator_speed() + 150.0) / 100.0;

        self.set_reactor_to_separator_flow(reactor_to_separator_flow);
        self.set_agitation_factor(agitation_factor);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use monjolo::dynamic_model::DynamicModel;
    use monjolo::snapshot::Snapshot;
    use monjolo::state_registry::StateRegistry;

    #[test]
    fn reactor_to_separator_flow_matches_hand_computed_value() {
        let registry = StateRegistry::shared();
        let (offered, _) = registry.borrow_mut().subscribe(
            &[
                "reactor.pressure",
                "reactor.vapor_composition.a",
                "reactor.vapor_composition.b",
                "reactor.vapor_composition.c",
                "reactor.vapor_composition.d",
                "reactor.vapor_composition.e",
                "reactor.vapor_composition.f",
                "reactor.vapor_composition.g",
                "reactor.vapor_composition.h",
                "separator.pressure",
                "agitator.speed",
            ],
            &[],
        );
        offered[0].set(750.0); // reactor.pressure
        offered[1].set(1.0); // reactor.vapor_composition.a = 100% componente A, evita mw=0
        offered[9].set(700.0); // separator.pressure
        offered[10].set(50.0); // agitator.speed

        let config = Snapshot::from_pairs(&[]);
        let flows = Flows::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        flows.evaluate();

        let (_, needed) = registry
            .borrow_mut()
            .subscribe(&[], &["flows.stream_flow.7", "flows.agitation_factor"]);
        registry.borrow_mut().resolve().expect("chaves já ofertadas deveriam resolver de novo sem erro");

        let constants = TepConstants::new();
        let expected_flow = 4574.21 * (750.0f64 - 700.0).max(0.0).sqrt() / constants.xmw[0];
        assert_eq!(needed[0].get(), expected_flow);
        assert_eq!(needed[1].get(), (50.0 + 150.0) / 100.0, "agitation_factor: (posição+150)/100");
    }

    #[test]
    fn evaluate_does_not_panic_with_realistic_pressures() {
        let registry = StateRegistry::shared();
        let (offered, _) = registry.borrow_mut().subscribe(
            &[
                "reactor.pressure",
                "reactor.vapor_composition.a",
                "reactor.vapor_composition.b",
                "reactor.vapor_composition.c",
                "reactor.vapor_composition.d",
                "reactor.vapor_composition.e",
                "reactor.vapor_composition.f",
                "reactor.vapor_composition.g",
                "reactor.vapor_composition.h",
                "separator.pressure",
                "agitator.speed",
            ],
            &[],
        );
        offered[0].set(2705.0); // reactor.pressure (docs/07-controle.md, operação normal)
        offered[1].set(0.485);
        offered[2].set(0.005);
        offered[3].set(0.51);
        offered[9].set(2633.7); // separator.pressure
        offered[10].set(22.1); // agitator.speed

        let config = Snapshot::from_pairs(&[]);
        let flows = Flows::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        flows.evaluate(); // sem NaN/panic
    }
}
