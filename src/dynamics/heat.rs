/* tep/dynamics/heat.rs */

/* Nominal — mesmo `s_zero` de TepDisturbanceState, canal 4 (TCWR, "reactor cooling water temp"). */
const REACTOR_COOLING_WATER_RETURN: f64 = 35.0;

/* Block 33 (separador) e Block 34 (stripper/condenser) saíram pra `dynamics::separator::heat`/
`dynamics::stripper::heat` (issue 10) — Heat fica só com Block 32 (reator), aguardando a migração
do Reactor (último passo).
*/
#[monjolo::dynamic_model(after = ["Flows"])]
pub struct Heat {
    #[need(key = "reactor.liquid_volume")]
    reactor_liquid_volume: f64,
    #[need(key = "reactor.temperature")]
    reactor_temperature: f64,
    #[need(key = "flows.agitation_factor")]
    agitation_factor: f64,

    #[offer(key = "heat.reactor_heat")]
    reactor_heat: f64,

    /* XMEAS(21) (Measured) lê este — publicado aqui porque é o mesmo nominal que `evaluate()` já
    usa internamente; sem isso, Measured precisaria duplicar a constante.
    */
    #[offer(key = "heat.reactor_cooling_water_return")]
    reactor_cooling_water_return: f64,
}

impl Heat {
    fn compute(&self) {
        let reactor_liquid_volume = self.reactor_liquid_volume();
        let reactor_temperature = self.reactor_temperature();
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

        self.set_reactor_heat(reactor_heat);
        self.set_reactor_cooling_water_return(REACTOR_COOLING_WATER_RETURN);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use monjolo::dynamic_model::DynamicModel;
    use monjolo::snapshot::Snapshot;
    use monjolo::state_registry::StateRegistry;

    #[test]
    fn reactor_heat_matches_hand_computed_value_in_the_uarlev_plateau() {
        let registry = StateRegistry::shared();
        let (offered, _) = registry.borrow_mut().subscribe(
            &["reactor.liquid_volume", "reactor.temperature", "flows.agitation_factor"],
            &[],
        );
        offered[0].set(60.0 * 7.8); // level = 60 > 50 -> uar_level = 1.0 (platô)
        offered[1].set(120.4);
        offered[2].set(1.7); // agitation_factor (AGSP a ~20%)

        let config = Snapshot::from_pairs(&[]);
        let heat = Heat::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        heat.evaluate();

        let (_, needed) = registry.borrow_mut().subscribe(&[], &["heat.reactor_heat"]);
        registry.borrow_mut().resolve().expect("chave já ofertada deveria resolver de novo sem erro");

        let uar = (-0.5 * 1.7 * 1.7 + 2.75 * 1.7 - 2.5) * 855490e-6;
        let expected = uar * (REACTOR_COOLING_WATER_RETURN - 120.4);
        assert_eq!(needed[0].get(), expected);
    }

    #[test]
    fn reactor_heat_is_zero_below_the_uarlev_floor() {
        let registry = StateRegistry::shared();
        let (offered, _) = registry.borrow_mut().subscribe(
            &["reactor.liquid_volume", "reactor.temperature", "flows.agitation_factor"],
            &[],
        );
        offered[0].set(5.0 * 7.8); // level = 5 < 10 -> uar_level = 0.0
        offered[1].set(120.4);
        offered[2].set(1.7);

        let config = Snapshot::from_pairs(&[]);
        let heat = Heat::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        heat.evaluate();

        let (_, needed) = registry.borrow_mut().subscribe(&[], &["heat.reactor_heat"]);
        registry.borrow_mut().resolve().expect("chave já ofertada deveria resolver de novo sem erro");
        assert_eq!(needed[0].get(), 0.0);
    }

    #[test]
    fn evaluate_does_not_panic_with_realistic_values() {
        let registry = StateRegistry::shared();
        let (offered, _) = registry.borrow_mut().subscribe(
            &["reactor.liquid_volume", "reactor.temperature", "flows.agitation_factor"],
            &[],
        );
        offered[0].set(103.3); // reactor.liquid_volume (docs/07-controle.md, nível normal)
        offered[1].set(120.4); // reactor.temperature
        offered[2].set(1.7); // agitation_factor (AGSP a ~20%)

        let config = Snapshot::from_pairs(&[]);
        let heat = Heat::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        heat.evaluate();
    }
}
