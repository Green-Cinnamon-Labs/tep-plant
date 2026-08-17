/* tep/dynamics/measured.rs */


/* `after = ["Derivatives"]`, não `["Heat"]` — Measured não lê nada que Derivatives escreve (só
`.derivative`, que Measured nunca consome), mas a cadeia única da fase (A) continua sendo um elo só
(ver `monjolo::component`, "Cadeia única, de propósito"): Derivatives é o novo último elo antes de
Measured, então é ele quem entra aqui agora.
*/
#[monjolo::dynamic_model(after = ["Derivatives"])]
pub struct Measured {
    #[need(key = "reactor.pressure")]
    reactor_pressure: f64,
    #[need(key = "reactor.liquid_volume")]
    reactor_liquid_volume: f64,
    #[need(key = "reactor.temperature")]
    reactor_temperature: f64,

    #[need(key = "separator.temperature")]
    separator_temperature: f64,
    #[need(key = "separator.liquid_volume")]
    separator_liquid_volume: f64,
    #[need(key = "separator.pressure")]
    separator_pressure: f64,
    #[need(key = "separator.liquid_density")]
    separator_liquid_density: f64,

    #[need(key = "stripper.liquid_volume")]
    stripper_liquid_volume: f64,
    #[need(key = "stripper.liquid_density")]
    stripper_liquid_density: f64,
    #[need(key = "stripper.temperature")]
    stripper_temperature: f64,

    #[need(key = "compressor.pressure")]
    compressor_pressure: f64,

    #[need(key = "flows.stream_flow.2")]
    a_feed_flow: f64,
    #[need(key = "flows.stream_flow.0")]
    d_feed_flow: f64,
    #[need(key = "flows.stream_flow.1")]
    e_feed_flow: f64,
    #[need(key = "flows.stream_flow.3")]
    ac_feed_flow: f64,
    #[need(key = "flows.stream_flow.8")]
    recycle_flow: f64,
    #[need(key = "flows.stream_flow.5")]
    reactor_feed_flow: f64,
    #[need(key = "flows.stream_flow.9")]
    purge_flow: f64,
    #[need(key = "flows.stream_flow.10")]
    separator_underflow_flow: f64,
    #[need(key = "flows.stream_flow.12")]
    stripper_product_flow: f64,
    #[need(key = "flows.d_feed_mol_weight")]
    d_feed_mol_weight: f64,
    #[need(key = "flows.e_feed_mol_weight")]
    e_feed_mol_weight: f64,
    #[need(key = "flows.compressor_work")]
    compressor_work: f64,

    #[need(key = "heat.condenser_heat")]
    condenser_heat: f64,
    #[need(key = "heat.reactor_cooling_water_return")]
    reactor_cooling_water_return: f64,
    #[need(key = "heat.separator_cooling_water_return")]
    separator_cooling_water_return: f64,

    #[offer(key = "xmeas.a_feed")]
    xmeas_a_feed: f64,
    #[offer(key = "xmeas.d_feed")]
    xmeas_d_feed: f64,
    #[offer(key = "xmeas.e_feed")]
    xmeas_e_feed: f64,
    #[offer(key = "xmeas.ac_feed")]
    xmeas_ac_feed: f64,
    #[offer(key = "xmeas.recycle_flow")]
    xmeas_recycle_flow: f64,
    #[offer(key = "xmeas.reactor_feed_rate")]
    xmeas_reactor_feed_rate: f64,
    #[offer(key = "xmeas.reactor_pressure")]
    xmeas_reactor_pressure: f64,
    #[offer(key = "xmeas.reactor_level")]
    xmeas_reactor_level: f64,
    #[offer(key = "xmeas.reactor_temperature")]
    xmeas_reactor_temperature: f64,
    #[offer(key = "xmeas.purge_rate")]
    xmeas_purge_rate: f64,
    #[offer(key = "xmeas.separator_temperature")]
    xmeas_separator_temperature: f64,
    #[offer(key = "xmeas.separator_level")]
    xmeas_separator_level: f64,
    #[offer(key = "xmeas.separator_pressure")]
    xmeas_separator_pressure: f64,
    #[offer(key = "xmeas.separator_underflow")]
    xmeas_separator_underflow: f64,
    #[offer(key = "xmeas.stripper_level")]
    xmeas_stripper_level: f64,
    #[offer(key = "xmeas.stripper_pressure")]
    xmeas_stripper_pressure: f64,
    #[offer(key = "xmeas.stripper_underflow")]
    xmeas_stripper_underflow: f64,
    #[offer(key = "xmeas.stripper_temperature")]
    xmeas_stripper_temperature: f64,
    #[offer(key = "xmeas.stripper_steam_flow")]
    xmeas_stripper_steam_flow: f64,
    #[offer(key = "xmeas.compressor_work")]
    xmeas_compressor_work: f64,
    #[offer(key = "xmeas.reactor_cooling_water_outlet_temp")]
    xmeas_reactor_cooling_water_outlet_temp: f64,
    #[offer(key = "xmeas.separator_cooling_water_outlet_temp")]
    xmeas_separator_cooling_water_outlet_temp: f64,

    /* Block 36 — 1.0 se qualquer condição de shutdown for verdadeira, 0.0 caso contrário. */
    #[offer(key = "xmeas.shutdown_detected")]
    shutdown_detected: f64,
}

impl Measured {
    fn compute(&self) {
        let reactor_pressure = self.reactor_pressure();
        let reactor_liquid_volume = self.reactor_liquid_volume();
        let reactor_temperature = self.reactor_temperature();
        let separator_liquid_volume = self.separator_liquid_volume();
        let separator_pressure = self.separator_pressure();
        let stripper_liquid_volume = self.stripper_liquid_volume();
        let stripper_temperature = self.stripper_temperature();
        let compressor_pressure = self.compressor_pressure();

        /* Block 35 — conversões de unidade preservadas exatamente do original: 0.359/35.3145
        (kmol/h → kscmh, gás), XMW*0.454 (kmol/h → kg/h, via peso molecular), (P-760)/760*101.325
        (mmHg gauge → kPa gauge), volume/densidade/35.3145 (kmol/h → m³/h via densidade molar).
        */
        let xmeas_a_feed = self.a_feed_flow() * 0.359 / 35.3145;
        let xmeas_d_feed = self.d_feed_flow() * self.d_feed_mol_weight() * 0.454;
        let xmeas_e_feed = self.e_feed_flow() * self.e_feed_mol_weight() * 0.454;
        let xmeas_ac_feed = self.ac_feed_flow() * 0.359 / 35.3145;
        let xmeas_recycle_flow = self.recycle_flow() * 0.359 / 35.3145;
        let xmeas_reactor_feed_rate = self.reactor_feed_flow() * 0.359 / 35.3145;
        let xmeas_reactor_pressure = (reactor_pressure - 760.0) / 760.0 * 101.325;
        let xmeas_reactor_level = (reactor_liquid_volume - 84.6) / 666.7 * 100.0;
        let xmeas_reactor_temperature = reactor_temperature;
        let xmeas_purge_rate = self.purge_flow() * 0.359 / 35.3145;
        let xmeas_separator_temperature = self.separator_temperature();
        let xmeas_separator_level = (separator_liquid_volume - 27.5) / 290.0 * 100.0;
        let xmeas_separator_pressure = (separator_pressure - 760.0) / 760.0 * 101.325;
        let xmeas_separator_underflow = self.separator_underflow_flow() / self.separator_liquid_density() / 35.3145;
        let xmeas_stripper_level = (stripper_liquid_volume - 78.25) / 156.5 * 100.0; /* VTC = 156.5 (TEINIT) */
        let xmeas_stripper_pressure = (compressor_pressure - 760.0) / 760.0 * 101.325;
        let xmeas_stripper_underflow = self.stripper_product_flow() / self.stripper_liquid_density() / 35.3145;
        let xmeas_stripper_temperature = stripper_temperature;
        let xmeas_stripper_steam_flow = self.condenser_heat() * 1.04e3 * 0.454;
        let xmeas_compressor_work = self.compressor_work() * 0.29307e3;
        let xmeas_reactor_cooling_water_outlet_temp = self.reactor_cooling_water_return();
        let xmeas_separator_cooling_water_outlet_temp = self.separator_cooling_water_return();

        /* Block 36 — detecção de shutdown: qualquer condição verdadeira já marca 1.0. Volumes em
        m³ convertidos de kmol via /35.3145 (mesma conversão usada em XMEAS 8/12/15, mas contra
        limites absolutos, não contra o zero de calibração dos instrumentos).
        */
        let shutdown_detected = xmeas_reactor_pressure > 3000.0
            || reactor_liquid_volume / 35.3145 > 24.0
            || reactor_liquid_volume / 35.3145 < 2.0
            || xmeas_reactor_temperature > 175.0
            || separator_liquid_volume / 35.3145 > 12.0
            || separator_liquid_volume / 35.3145 < 1.0
            || stripper_liquid_volume / 35.3145 > 8.0
            || stripper_liquid_volume / 35.3145 < 1.0;

        self.set_xmeas_a_feed(xmeas_a_feed);
        self.set_xmeas_d_feed(xmeas_d_feed);
        self.set_xmeas_e_feed(xmeas_e_feed);
        self.set_xmeas_ac_feed(xmeas_ac_feed);
        self.set_xmeas_recycle_flow(xmeas_recycle_flow);
        self.set_xmeas_reactor_feed_rate(xmeas_reactor_feed_rate);
        self.set_xmeas_reactor_pressure(xmeas_reactor_pressure);
        self.set_xmeas_reactor_level(xmeas_reactor_level);
        self.set_xmeas_reactor_temperature(xmeas_reactor_temperature);
        self.set_xmeas_purge_rate(xmeas_purge_rate);
        self.set_xmeas_separator_temperature(xmeas_separator_temperature);
        self.set_xmeas_separator_level(xmeas_separator_level);
        self.set_xmeas_separator_pressure(xmeas_separator_pressure);
        self.set_xmeas_separator_underflow(xmeas_separator_underflow);
        self.set_xmeas_stripper_level(xmeas_stripper_level);
        self.set_xmeas_stripper_pressure(xmeas_stripper_pressure);
        self.set_xmeas_stripper_underflow(xmeas_stripper_underflow);
        self.set_xmeas_stripper_temperature(xmeas_stripper_temperature);
        self.set_xmeas_stripper_steam_flow(xmeas_stripper_steam_flow);
        self.set_xmeas_compressor_work(xmeas_compressor_work);
        self.set_xmeas_reactor_cooling_water_outlet_temp(xmeas_reactor_cooling_water_outlet_temp);
        self.set_xmeas_separator_cooling_water_outlet_temp(xmeas_separator_cooling_water_outlet_temp);
        self.set_shutdown_detected(if shutdown_detected { 1.0 } else { 0.0 });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use monjolo::dynamic_model::DynamicModel;
    use monjolo::snapshot::Snapshot;
    use monjolo::state_registry::StateRegistry;

    fn seed_all(registry: &mut StateRegistry) -> Vec<monjolo::state_registry::Proxy> {
        let (offered, _) = registry.subscribe(
            &[
                "reactor.pressure", "reactor.liquid_volume", "reactor.temperature",
                "separator.temperature", "separator.liquid_volume", "separator.pressure",
                "separator.liquid_density",
                "stripper.liquid_volume", "stripper.liquid_density", "stripper.temperature",
                "compressor.pressure",
                "flows.stream_flow.2", "flows.stream_flow.0", "flows.stream_flow.1",
                "flows.stream_flow.3", "flows.stream_flow.8", "flows.stream_flow.5",
                "flows.stream_flow.9", "flows.stream_flow.10", "flows.stream_flow.12",
                "flows.d_feed_mol_weight", "flows.e_feed_mol_weight", "flows.compressor_work",
                "heat.condenser_heat", "heat.reactor_cooling_water_return",
                "heat.separator_cooling_water_return",
            ],
            &[],
        );
        offered
    }

    #[test]
    fn reactor_pressure_conversion_matches_hand_computed_value() {
        let registry = StateRegistry::shared();
        let offered = seed_all(&mut registry.borrow_mut());
        offered[0].set(2705.0); // reactor.pressure [mmHg]
        // separator.liquid_density/stripper.liquid_density não podem ser 0.0 (usadas como
        // divisor) — semeia com algo plausível pra não gerar NaN/inf nos outros XMEAS.
        offered[6].set(35.0); // separator.liquid_density
        offered[8].set(35.0); // stripper.liquid_density

        let config = Snapshot::from_pairs(&[]);
        let measured = Measured::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        measured.evaluate();

        let (_, needed) = registry.borrow_mut().subscribe(&[], &["xmeas.reactor_pressure"]);
        registry.borrow_mut().resolve().expect("chave já ofertada deveria resolver de novo sem erro");
        assert_eq!(needed[0].get(), (2705.0 - 760.0) / 760.0 * 101.325);
    }

    #[test]
    fn shutdown_detected_flags_reactor_pressure_above_3000_kpa() {
        let registry = StateRegistry::shared();
        let offered = seed_all(&mut registry.borrow_mut());
        offered[0].set(760.0 + 3000.0 / 101.325 * 760.0 + 1.0); // garante xmeas.reactor_pressure > 3000
        offered[6].set(35.0);
        offered[8].set(35.0);

        let config = Snapshot::from_pairs(&[]);
        let measured = Measured::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        measured.evaluate();

        let (_, needed) = registry.borrow_mut().subscribe(&[], &["xmeas.shutdown_detected"]);
        registry.borrow_mut().resolve().expect("chave já ofertada deveria resolver de novo sem erro");
        assert_eq!(needed[0].get(), 1.0);
    }

    #[test]
    fn shutdown_not_detected_within_normal_operating_ranges() {
        let registry = StateRegistry::shared();
        let offered = seed_all(&mut registry.borrow_mut());
        offered[0].set(2705.0); // reactor.pressure — dentro do normal (docs/07-controle.md)
        offered[1].set(12.0 * 35.3145); // reactor.liquid_volume — 12 m³ convertido, dentro de [2,24]
        offered[2].set(120.4); // reactor.temperature — dentro de <175
        offered[4].set(6.0 * 35.3145); // separator.liquid_volume — 6 m³, dentro de [1,12]
        offered[6].set(35.0);
        offered[7].set(4.0 * 35.3145); // stripper.liquid_volume — 4 m³, dentro de [1,8]
        offered[8].set(35.0);

        let config = Snapshot::from_pairs(&[]);
        let measured = Measured::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        measured.evaluate();

        let (_, needed) = registry.borrow_mut().subscribe(&[], &["xmeas.shutdown_detected"]);
        registry.borrow_mut().resolve().expect("chave já ofertada deveria resolver de novo sem erro");
        assert_eq!(needed[0].get(), 0.0);
    }
}
