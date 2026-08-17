/* tep/dynamics/heat.rs */


/* Nominal — mesmo `s_zero` de TepDisturbanceState, canal 4 (TCWR, "reactor cooling water temp"). */
const REACTOR_COOLING_WATER_RETURN: f64 = 35.0;
/* Nominal — mesmo `s_zero` de TepDisturbanceState, canal 5 (TCWS, "condenser cooling water temp"). */
const SEPARATOR_COOLING_WATER_RETURN: f64 = 40.0;

#[monjolo::dynamic_model(after = ["Flows"])]
pub struct Heat {
    #[need(key = "reactor.liquid_volume")]
    reactor_liquid_volume: f64,
    #[need(key = "reactor.temperature")]
    reactor_temperature: f64,
    #[need(key = "stripper.temperature")]
    stripper_temperature: f64,

    #[need(key = "flows.agitation_factor")]
    agitation_factor: f64,
    #[need(key = "flows.stream_flow.7")]
    reactor_to_separator_flow: f64,
    #[need(key = "flows.condenser_ua")]
    condenser_ua: f64,

    #[offer(key = "heat.reactor_heat")]
    reactor_heat: f64,
    #[offer(key = "heat.separator_heat")]
    separator_heat: f64,
    #[offer(key = "heat.condenser_heat")]
    condenser_heat: f64,

    /* XMEAS(21)/XMEAS(22) (Measured) leem estes dois — publicados aqui porque são os mesmos
    nominais que `evaluate()` já usa internamente (ver comentário do topo do arquivo); sem isso,
    Measured precisaria duplicar as duas constantes.
    */
    #[offer(key = "heat.reactor_cooling_water_return")]
    reactor_cooling_water_return: f64,
    #[offer(key = "heat.separator_cooling_water_return")]
    separator_cooling_water_return: f64,
}

impl Heat {
    fn compute(&self) {
        let reactor_liquid_volume = self.reactor_liquid_volume();
        let reactor_temperature = self.reactor_temperature();
        let stripper_temperature = self.stripper_temperature();
        let agitation_factor = self.agitation_factor();

        /* Block 32: troca térmica no reator — UARLEV degrau/rampa/platô conforme o nível de
        líquido (7.8 = fator de conversão de volume pra "nível" usado neste bloco especificamente,
        igual ao original).
        */
        let level = reactor_liquid_volume / 7.8;
        let uar_level = if level > 50.0 {
            1.0
        } else if level < 10.0 {
            0.0
        } else {
            0.025 * level - 0.25
        };
        let uar = uar_level * (-0.5 * agitation_factor * agitation_factor + 2.75 * agitation_factor - 2.5) * 855490e-6;
        let reactor_heat = uar * (REACTOR_COOLING_WATER_RETURN - reactor_temperature) * (1.0 - 0.35 * 0.0);

        /* Block 33: troca térmica no separador — UAS depende da vazão reator→separador
        (flows.stream_flow.7); a temperatura de referência é a do REATOR (não do separador — TST(8)
        aponta pro reator no teprob.f, Block 20), preservado por fidelidade.
        */
        let uas = 0.404655 * (1.0 - 1.0 / (1.0 + (self.reactor_to_separator_flow() / 3528.73).powi(4)));
        let separator_heat = uas * (SEPARATOR_COOLING_WATER_RETURN - reactor_temperature) * (1.0 - 0.25 * 0.0);

        /* Block 34: resfriamento condicional (reboiler do stripper) — só troca calor se a
        temperatura do stripper estiver abaixo de 100°C (ponto de ebulição da água a 1 atm).
        */
        let condenser_heat = if stripper_temperature < 100.0 {
            self.condenser_ua() * (100.0 - stripper_temperature)
        } else {
            0.0
        };

        self.set_reactor_heat(reactor_heat);
        self.set_separator_heat(separator_heat);
        self.set_condenser_heat(condenser_heat);
        self.set_reactor_cooling_water_return(REACTOR_COOLING_WATER_RETURN);
        self.set_separator_cooling_water_return(SEPARATOR_COOLING_WATER_RETURN);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use monjolo::dynamic_model::DynamicModel;
    use monjolo::snapshot::Snapshot;
    use monjolo::state_registry::StateRegistry;

    #[test]
    fn condenser_heat_is_zero_above_boiling_point() {
        let registry = StateRegistry::shared();
        let (offered, _) = registry.borrow_mut().subscribe(
            &[
                "reactor.liquid_volume", "reactor.temperature", "stripper.temperature",
                "flows.agitation_factor", "flows.stream_flow.7", "flows.condenser_ua",
            ],
            &[],
        );
        offered[2].set(120.0); // stripper.temperature > 100 → sem troca
        offered[5].set(500.0); // condenser_ua irrelevante nesse caso

        let config = Snapshot::from_pairs(&[]);
        let heat = Heat::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        heat.evaluate();

        let (_, needed) = registry.borrow_mut().subscribe(&[], &["heat.condenser_heat"]);
        registry.borrow_mut().resolve().expect("chave já ofertada deveria resolver de novo sem erro");
        assert_eq!(needed[0].get(), 0.0);
    }

    #[test]
    fn condenser_heat_scales_with_ua_and_temperature_gap_below_boiling_point() {
        let registry = StateRegistry::shared();
        let (offered, _) = registry.borrow_mut().subscribe(
            &[
                "reactor.liquid_volume", "reactor.temperature", "stripper.temperature",
                "flows.agitation_factor", "flows.stream_flow.7", "flows.condenser_ua",
            ],
            &[],
        );
        offered[2].set(65.0); // stripper.temperature < 100
        offered[5].set(10.0); // condenser_ua

        let config = Snapshot::from_pairs(&[]);
        let heat = Heat::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        heat.evaluate();

        let (_, needed) = registry.borrow_mut().subscribe(&[], &["heat.condenser_heat"]);
        registry.borrow_mut().resolve().expect("chave já ofertada deveria resolver de novo sem erro");
        assert_eq!(needed[0].get(), 10.0 * (100.0 - 65.0), "condenser_ua * (100 - stripper.temperature)");
    }

    #[test]
    fn evaluate_does_not_panic_with_realistic_values() {
        let registry = StateRegistry::shared();
        let (offered, _) = registry.borrow_mut().subscribe(
            &[
                "reactor.liquid_volume", "reactor.temperature", "stripper.temperature",
                "flows.agitation_factor", "flows.stream_flow.7", "flows.condenser_ua",
            ],
            &[],
        );
        offered[0].set(103.3); // reactor.liquid_volume (docs/07-controle.md, nível normal)
        offered[1].set(120.4); // reactor.temperature
        offered[2].set(65.7); // stripper.temperature
        offered[3].set(1.7); // agitation_factor (AGSP a ~20% = (20+150)/100)
        offered[4].set(9077.5); // flows.stream_flow.7, reator->separador em operação normal
        offered[5].set(9.5); // flows.condenser_ua

        let config = Snapshot::from_pairs(&[]);
        let heat = Heat::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        heat.evaluate();
    }
}
