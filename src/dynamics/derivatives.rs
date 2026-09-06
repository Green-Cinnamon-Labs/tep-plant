/* tep/dynamics/derivatives.rs */

/** Só sobra a seção "Reator" do Block 40 de teprob.f (YP(1..9)) — Separator/Stripper/Compressor já
têm seu próprio `yp_derivative` (issue 10); esta é a última EDO ainda não migrada, aguardando o
passo final (Reactor). `after = ["Heat"]` continua necessário: `heat.reactor_heat` só existe depois
de `Heat::compute()` rodar.

Mapeamento de streams preservado de quando este componente cobria as 4 unidades: os slots
(`flows.stream_flow.N`) são exatamente `FTM(N+1)` do original — sem reordenação, só o deslocamento
de índice 1→0.
*/

use crate::physics::constants::TepConstants;
use monjolo::chemistry::mixture_enthalpy;

#[monjolo::dynamic_model(after = ["Heat"])]
pub struct Derivatives {
    #[need(key = "reactor.temperature")]
    reactor_temperature: f64,
    #[need(prefix = "reactor.vapor_composition", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    reactor_vapor_composition: [f64; 8],
    #[need(prefix = "reactor.reaction_rates", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    reactor_reaction_rates: [f64; 8],
    #[need(key = "reactor.heat_of_reaction")]
    reactor_heat_of_reaction: f64,

    #[need(key = "compressor.temperature")]
    compressor_temperature: f64,
    #[need(prefix = "compressor.vapor_composition", components = ["0", "1", "2", "3", "4", "5", "6", "7"])]
    compressor_vapor_composition: [f64; 8],

    /* Só os slots 6 (recycle do compressor→reator) e 7 (reator→separador) — os únicos que o
    balanço do Reator consome.
    */
    #[need(key = "flows.stream_flow.6")]
    compressor_recycle_flow: f64,
    #[need(key = "flows.stream_flow.7")]
    reactor_outlet_flow: f64,

    #[need(key = "heat.reactor_heat")]
    reactor_heat: f64,

    #[offer(prefix = "reactor.state", components = ["vapor_a.derivative", "vapor_b.derivative", "vapor_c.derivative"])]
    reactor_vapor_derivative: [f64; 3],
    #[offer(prefix = "reactor.state", components = ["liquid_d.derivative", "liquid_e.derivative", "liquid_f.derivative", "liquid_g.derivative", "liquid_h.derivative"])]
    reactor_liquid_derivative: [f64; 5],
    #[offer(key = "reactor.state.enthalpy.derivative")]
    reactor_enthalpy_derivative: f64,
}

impl Derivatives {
    fn compute(&self) {
        let constants = TepConstants::new();

        let reactor_vapor = self.reactor_vapor_composition();
        let reactor_temperature = self.reactor_temperature();
        let compressor_vapor = self.compressor_vapor_composition();
        let compressor_temperature = self.compressor_temperature();

        let flow6 = self.compressor_recycle_flow();
        let flow7 = self.reactor_outlet_flow();

        /* slot 6 (recycle do compressor, cópia do slot 5) e slot 7 (saída pro separador) —
        entalpias reconstruídas frescas de composição+temperatura já publicadas.
        */
        let enthalpy_compressor_recycle = mixture_enthalpy(&compressor_vapor, compressor_temperature, 1, &constants);
        let enthalpy_reactor_outlet = mixture_enthalpy(&reactor_vapor, reactor_temperature, 1, &constants);

        /* ===== Reator — YP(1..8), YP(9): FCM(·,7) - FCM(·,8) + CRXR(·) ===== */
        let reaction_rates = self.reactor_reaction_rates();
        let mut reactor_vapor_derivative = [0.0f64; 3];
        let mut reactor_liquid_derivative = [0.0f64; 5];
        for i in 0..8 {
            let value = compressor_vapor[i] * flow6 - reactor_vapor[i] * flow7 + reaction_rates[i];
            if i < 3 {
                reactor_vapor_derivative[i] = value;
            } else {
                reactor_liquid_derivative[i - 3] = value;
            }
        }
        let reactor_enthalpy_derivative = enthalpy_compressor_recycle * flow6
            - enthalpy_reactor_outlet * flow7
            + self.reactor_heat_of_reaction()
            + self.reactor_heat();

        self.set_reactor_vapor_derivative(reactor_vapor_derivative);
        self.set_reactor_liquid_derivative(reactor_liquid_derivative);
        self.set_reactor_enthalpy_derivative(reactor_enthalpy_derivative);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use monjolo::dynamic_model::DynamicModel;
    use monjolo::snapshot::Snapshot;
    use monjolo::state_registry::{Proxy, StateRegistry};

    struct Seeded {
        reactor_vapor_composition: [Proxy; 8],
        reactor_reaction_rates: [Proxy; 8],
        reactor_heat_of_reaction: Proxy,
        compressor_temperature: Proxy,
        compressor_vapor_composition: [Proxy; 8],
        compressor_recycle_flow: Proxy,
        reactor_outlet_flow: Proxy,
        reactor_heat: Proxy,
    }

    fn array8(offered: &[Proxy], start: usize) -> [Proxy; 8] {
        std::array::from_fn(|i| offered[start + i].clone())
    }

    fn seed_registry(registry: &mut StateRegistry) -> Seeded {
        let mut keys: Vec<String> = Vec::new();
        keys.push("reactor.temperature".to_string());
        let reactor_vapor_composition_start = keys.len();
        for c in ["a", "b", "c", "d", "e", "f", "g", "h"] { keys.push(format!("reactor.vapor_composition.{c}")); }
        let reactor_reaction_rates_start = keys.len();
        for c in ["a", "b", "c", "d", "e", "f", "g", "h"] { keys.push(format!("reactor.reaction_rates.{c}")); }
        let reactor_heat_of_reaction_idx = keys.len();
        keys.push("reactor.heat_of_reaction".to_string());

        let compressor_temperature_idx = keys.len();
        keys.push("compressor.temperature".to_string());
        let compressor_vapor_composition_start = keys.len();
        for c in ["0", "1", "2", "3", "4", "5", "6", "7"] { keys.push(format!("compressor.vapor_composition.{c}")); }

        let compressor_recycle_flow_idx = keys.len();
        keys.push("flows.stream_flow.6".to_string());
        let reactor_outlet_flow_idx = keys.len();
        keys.push("flows.stream_flow.7".to_string());

        let reactor_heat_idx = keys.len();
        keys.push("heat.reactor_heat".to_string());

        let key_refs: Vec<&str> = keys.iter().map(String::as_str).collect();
        let (offered, _) = registry.subscribe(&key_refs, &[]);

        Seeded {
            reactor_vapor_composition: array8(&offered, reactor_vapor_composition_start),
            reactor_reaction_rates: array8(&offered, reactor_reaction_rates_start),
            reactor_heat_of_reaction: offered[reactor_heat_of_reaction_idx].clone(),
            compressor_temperature: offered[compressor_temperature_idx].clone(),
            compressor_vapor_composition: array8(&offered, compressor_vapor_composition_start),
            compressor_recycle_flow: offered[compressor_recycle_flow_idx].clone(),
            reactor_outlet_flow: offered[reactor_outlet_flow_idx].clone(),
            reactor_heat: offered[reactor_heat_idx].clone(),
        }
    }

    fn read_back(registry: &mut StateRegistry, keys: &[&str]) -> Vec<Proxy> {
        let (_, needed) = registry.subscribe(&[], keys);
        registry.resolve().expect("chaves já ofertadas deveriam resolver de novo sem erro");
        needed
    }

    #[test]
    fn reactor_mass_derivative_reflects_inflow_and_outflow_terms() {
        let registry = StateRegistry::shared();
        let seeded = seed_registry(&mut registry.borrow_mut());
        seeded.compressor_vapor_composition[0].set(2.0); // componente a, entra via flow6
        seeded.compressor_recycle_flow.set(3.0);
        seeded.reactor_vapor_composition[3].set(3.0); // componente d, sai via flow7
        seeded.reactor_outlet_flow.set(4.0);

        let config = Snapshot::from_pairs(&[]);
        let derivatives = Derivatives::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        derivatives.evaluate();

        let out = read_back(
            &mut registry.borrow_mut(),
            &["reactor.state.vapor_a.derivative", "reactor.state.liquid_d.derivative"],
        );
        assert_eq!(out[0].get(), 2.0 * 3.0, "entrada: compressor_vapor[a]*flow6, CRXR(a)=0");
        assert_eq!(out[1].get(), -3.0 * 4.0, "saída: -reactor_vapor[d]*flow7, CRXR(d)=0");
    }

    #[test]
    fn reactor_mass_derivative_includes_reaction_rate_term() {
        let registry = StateRegistry::shared();
        let seeded = seed_registry(&mut registry.borrow_mut());
        // flow6/flow7 ficam em 0.0 — isola CRXR puro
        seeded.reactor_reaction_rates[0].set(1.5);
        seeded.reactor_reaction_rates[3].set(-0.7);

        let config = Snapshot::from_pairs(&[]);
        let derivatives = Derivatives::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        derivatives.evaluate();

        let out = read_back(
            &mut registry.borrow_mut(),
            &["reactor.state.vapor_a.derivative", "reactor.state.liquid_d.derivative"],
        );
        assert_eq!(out[0].get(), 1.5);
        assert_eq!(out[1].get(), -0.7);
    }

    #[test]
    fn reactor_energy_derivative_isolates_heat_terms_when_flows_are_zero() {
        let registry = StateRegistry::shared();
        let seeded = seed_registry(&mut registry.borrow_mut());
        // flow6/flow7 ficam em 0.0 — zera os dois termos de entalpia*vazão
        seeded.reactor_heat_of_reaction.set(11.0);
        seeded.reactor_heat.set(5.0);

        let config = Snapshot::from_pairs(&[]);
        let derivatives = Derivatives::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        derivatives.evaluate();

        let out = read_back(&mut registry.borrow_mut(), &["reactor.state.enthalpy.derivative"]);
        assert_eq!(out[0].get(), 11.0 + 5.0);
    }

    #[test]
    fn evaluate_does_not_panic_with_realistic_operating_values() {
        let registry = StateRegistry::shared();
        let seeded = seed_registry(&mut registry.borrow_mut());

        seeded.reactor_vapor_composition[0].set(1.0);
        for r in &seeded.reactor_reaction_rates { r.set(0.5); }
        seeded.reactor_heat_of_reaction.set(-200.0);

        seeded.compressor_temperature.set(95.3);
        seeded.compressor_vapor_composition[0].set(1.0);
        seeded.compressor_recycle_flow.set(20.0);
        seeded.reactor_outlet_flow.set(20.0);

        seeded.reactor_heat.set(100.0);

        let config = Snapshot::from_pairs(&[]);
        let derivatives = Derivatives::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        derivatives.evaluate();

        let keys = [
            "reactor.state.vapor_a.derivative",
            "reactor.state.vapor_b.derivative",
            "reactor.state.vapor_c.derivative",
            "reactor.state.liquid_d.derivative",
            "reactor.state.liquid_e.derivative",
            "reactor.state.liquid_f.derivative",
            "reactor.state.liquid_g.derivative",
            "reactor.state.liquid_h.derivative",
            "reactor.state.enthalpy.derivative",
        ];
        let out = read_back(&mut registry.borrow_mut(), &keys);
        for proxy in &out {
            assert!(proxy.get().is_finite(), "derivada não pode ser NaN/Inf com valores realistas de operação");
        }
    }
}
