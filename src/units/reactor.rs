/* Documentação: docs/12-reator.md */

use crate::physics::constants::{TepConstants, TEP_SPECIES};
use monjolo::chemistry::{arrhenius, liquid_density, temperature_from_enthalpy, Mixture, Reactions};
use std::sync::LazyLock;

const REACTOR_VOLUME: f64 = 1300.0;
const REACTION_FACTOR_1_NOMINAL: f64 = 1.0;
const REACTION_FACTOR_2_NOMINAL: f64 = 1.0;
const TEMPERATURE_SEED: f64 = 120.0;
const REACTOR_COOLING_WATER_RETURN: f64 = 94.59927549;

static REACTIONS: LazyLock<Reactions<8>> = LazyLock::new(|| {
    Reactions::new(&TEP_SPECIES)
        .add(
            "A + C + D -> G", 
            0.06899381054, 
            |temperature_k, p| {
                let (a, c, d) = (p.get("A"), p.get("C"), p.get("D"));
                if a > 0.0 && c > 0.0 {
                    arrhenius(31.5859536, 40000.0, temperature_k) * REACTION_FACTOR_1_NOMINAL * (a.powf(1.1544) * c.powf(0.3735) * d)
                } else {
                    0.0
                }
        })
        .add(
            "A + C + E -> H", 
            0.05, 
            |temperature_k, p| {
                let (a, c, e) = (p.get("A"), p.get("C"), p.get("E"));
                if a > 0.0 && c > 0.0 {
                    arrhenius(3.00094014, 20000.0, temperature_k) * REACTION_FACTOR_2_NOMINAL * (a.powf(1.1544) * c.powf(0.3735) * e)
                } else {
                    0.0
                }
        })
        .add(
            "A + E -> F", 
            0.0, 
            |temperature_k, p| arrhenius(53.4060443, 60000.0, temperature_k) * (p.get("A") * p.get("E")))
        .add(
            "1.5 D -> F", 
            0.0, 
            |temperature_k, p| { arrhenius(53.4060443, 60000.0, temperature_k) * 0.767488334 * (p.get("A") * p.get("D"))
        })
});

#[monjolo::dynamic_model(tasks)]
pub struct Reactor {
    #[state]
    #[config(prefix = "state.reactor_vapor", components = ["A", "B", "C"])]
    #[offer(prefix = "reactor.state", components = ["vapor_a", "vapor_b", "vapor_c"])]
    vapor: [f64; 3],

    #[state]
    #[config(prefix = "state.reactor_vapor", components = ["D", "E", "F", "G", "H"])]
    #[offer(prefix = "reactor.state", components = ["liquid_d", "liquid_e", "liquid_f", "liquid_g", "liquid_h"])]
    liquid: [f64; 5],

    #[state]
    #[config(key = "state.reactor.energy")]
    #[offer(key = "reactor.state.enthalpy")]
    enthalpy: f64,

    constants: TepConstants,
}

#[monjolo::tasks(species = TEP_SPECIES, len = 8)]
impl Reactor {

    #[task]
    fn physical_state(&self) {
        let vapor = Mixture::at(0, &self.vapor(), &TEP_SPECIES);
        let liquid = Mixture::at(3, &self.liquid(), &TEP_SPECIES);
        let liquid_composition = liquid.mole_fractions();

        let specific_enthalpy = self.enthalpy() / liquid.total();
        let temperature = temperature_from_enthalpy(&liquid_composition.as_array(), TEMPERATURE_SEED, specific_enthalpy, 0, &self.constants);
        let temperature_k = temperature + 273.15;
        let density = liquid_density(&liquid_composition.as_array(), temperature, &self.constants);
        let volume_liquid = liquid.total() / density;
        let volume_vapor = REACTOR_VOLUME - volume_liquid;

        let partial_pressures = vapor.ideal_gas_pressure(temperature_k, volume_vapor) + liquid_composition.vapor_pressure(temperature, &self.constants);
        let pressure = partial_pressures.total();
        let vapor_composition = partial_pressures.mole_fractions();

        let reacted = REACTIONS.at(temperature_k, &partial_pressures, volume_vapor);

        offer::reactor__temperature = temperature;
        offer::reactor__pressure = pressure;
        offer::reactor__liquid_volume = volume_liquid;
        offer::reactor__heat_of_reaction = reacted.heat;
        offer::reactor__vapor_composition::<Mixture> = vapor_composition;
        offer::reactor__reaction_rates::<Mixture> = reacted.species_rates;
    }

    #[task]
    fn flow_to_separator(&self) {
        let mol_weight = need::reactor__vapor_composition::<Mixture>.dot(&self.constants.xmw);
        offer::flows__stream_flow__7 = 4574.21 * (need::reactor__pressure - need::separator__pressure).max(0.0).sqrt() * (1.0 - 0.25 * 0.0) / mol_weight;
    }

    #[task]
    fn heat_exchange(&self) {
        let agitation_factor = (need::agitator__speed + 150.0) / 100.0;
        let level = need::reactor__liquid_volume / 7.8;
        let uar_level = if level > 50.0 {
            1.0
        } else if level < 10.0 {
            0.0
        } else {
            0.025 * level - 0.25
        };
        let uar = uar_level * (-0.5 * agitation_factor * agitation_factor + 2.75 * agitation_factor - 2.5) * 855490e-6;
        let twr = REACTOR_COOLING_WATER_RETURN;

        offer::heat__reactor_heat = uar * (twr - need::reactor__temperature) * (1.0 - 0.35 * 0.0);
        offer::heat__reactor_cooling_water_return = twr;
    }

    #[task]
    fn mass_and_energy_balance(&self) {
        let compressor_vapor = need::compressor__vapor_composition::<Mixture>;
        let reactor_vapor = need::reactor__vapor_composition::<Mixture>;
        let compressor_recycle_flow = need::flows__stream_flow__6;
        let outlet_flow = need::flows__stream_flow__7;

        let derivative = compressor_vapor.scaled_by(compressor_recycle_flow) - reactor_vapor.scaled_by(outlet_flow) + need::reactor__reaction_rates::<Mixture>;

        offer::reactor__state__vapor_a__derivative = derivative.component(0);
        offer::reactor__state__vapor_b__derivative = derivative.component(1);
        offer::reactor__state__vapor_c__derivative = derivative.component(2);
        offer::reactor__state__liquid_d__derivative = derivative.component(3);
        offer::reactor__state__liquid_e__derivative = derivative.component(4);
        offer::reactor__state__liquid_f__derivative = derivative.component(5);
        offer::reactor__state__liquid_g__derivative = derivative.component(6);
        offer::reactor__state__liquid_h__derivative = derivative.component(7);
        offer::reactor__state__enthalpy__derivative = compressor_vapor.enthalpy(need::compressor__temperature, 1, &self.constants) * compressor_recycle_flow
            - reactor_vapor.enthalpy(need::reactor__temperature, 1, &self.constants) * outlet_flow
            + need::reactor__heat_of_reaction
            + need::heat__reactor_heat;
    }

    #[task]
    fn xmeas_readings(&self) {
        offer::xmeas__reactor__pressure = (need::reactor__pressure - 760.0) / 760.0 * 101.325;
        offer::xmeas__reactor__level = (need::reactor__liquid_volume - 84.6) / 666.7 * 100.0;
        offer::xmeas__reactor__temperature = need::reactor__temperature;
        offer::xmeas__reactor__cooling_water_outlet_temperature = need::heat__reactor_cooling_water_return;
    }
}
