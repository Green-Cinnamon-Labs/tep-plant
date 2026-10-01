/* Documentação: docs/13-separador.md */

use crate::physics::constants::{TepConstants, TEP_SPECIES};
use monjolo::chemistry::{liquid_density, temperature_from_enthalpy, Mixture};

const SEPARATOR_VOLUME: f64 = 3500.0;
const SEPARATOR_UNDERFLOW_RANGE: f64 = 1500.0;
const SEPARATOR_COOLING_WATER_RETURN: f64 = 77.29698353;

#[monjolo::dynamic_model(tasks)]
pub struct Separator {
    #[state]
    #[config(prefix = "state.separator_vapor", components = ["A", "B", "C"])]
    #[offer(prefix = "separator.state", components = ["vapor_a", "vapor_b", "vapor_c"])]
    vapor: [f64; 3],

    #[state]
    #[config(prefix = "state.separator_vapor", components = ["D", "E", "F", "G", "H"])]
    #[offer(prefix = "separator.state", components = ["liquid_d", "liquid_e", "liquid_f", "liquid_g", "liquid_h"])]
    liquid: [f64; 5],

    #[state]
    #[config(key = "state.separator.energy")]
    #[offer(key = "separator.state.enthalpy")]
    enthalpy: f64,

    constants: TepConstants,
}

#[monjolo::tasks(species = TEP_SPECIES, len = 8)]
impl Separator {
    #[task]
    fn physical_state(&self) {
        let vapor = Mixture::at(0, &self.vapor(), &TEP_SPECIES);
        let liquid = Mixture::at(3, &self.liquid(), &TEP_SPECIES);
        let liquid_composition = liquid.mole_fractions();

        let specific_enthalpy = self.enthalpy() / liquid.total();
        let temperature = temperature_from_enthalpy(&liquid_composition.as_array(), need::reactor__temperature, specific_enthalpy, 0, &self.constants);
        let temperature_k = temperature + 273.15;
        let density = liquid_density(&liquid_composition.as_array(), temperature, &self.constants);
        let volume_liquid = liquid.total() / density;
        let volume_vapor = SEPARATOR_VOLUME - volume_liquid;

        let partial_pressures = vapor.ideal_gas_pressure(temperature_k, volume_vapor)
            + liquid_composition.vapor_pressure(temperature, &self.constants);
        let pressure = partial_pressures.total();
        let vapor_composition = partial_pressures.mole_fractions();

        offer::separator__temperature = temperature;
        offer::separator__pressure = pressure;
        offer::separator__liquid_volume = volume_liquid;
        offer::separator__liquid_density = density;
        offer::separator__liquid_composition::<Mixture> = liquid_composition;
        offer::separator__vapor_composition::<Mixture> = vapor_composition;
    }

    #[task]
    fn outlet_flows(&self) {
        let mol_weight = need::separator__vapor_composition::<Mixture>.dot(&self.constants.xmw);
        offer::flows__stream_flow__9 = need::valve__purge__position * 0.151169 * (need::separator__pressure - 760.0).max(0.0).sqrt() / mol_weight;
        offer::flows__stream_flow__10 = need::valve__separator_underflow__position * SEPARATOR_UNDERFLOW_RANGE / 100.0;
    }

    #[task]
    fn heat_exchange(&self) {
        let uas = 0.404655 * (1.0 - 1.0 / (1.0 + (need::flows__stream_flow__7 / 3528.73).powi(4)));
        offer::heat__separator_heat = uas * (SEPARATOR_COOLING_WATER_RETURN - need::reactor__temperature) * (1.0 - 0.25 * 0.0);
        offer::heat__separator_cooling_water_return = SEPARATOR_COOLING_WATER_RETURN;
    }

    #[task]
    fn mass_and_energy_balance(&self) {
        let reactor_vapor = need::reactor__vapor_composition::<Mixture>;
        let separator_vapor = need::separator__vapor_composition::<Mixture>;
        let separator_liquid = need::separator__liquid_composition::<Mixture>;
        let flow7 = need::flows__stream_flow__7;
        let flow8 = need::flows__stream_flow__8;
        let flow9 = need::flows__stream_flow__9;
        let flow10 = need::flows__stream_flow__10;
        let separator_temperature = need::separator__temperature;

        let enthalpy_reactor_outlet = reactor_vapor.enthalpy(need::reactor__temperature, 1, &self.constants);
        let enthalpy_separator_vapor_uncorrected = separator_vapor.enthalpy(separator_temperature, 1, &self.constants);
        let enthalpy_separator_liquid = separator_liquid.enthalpy(separator_temperature, 0, &self.constants);

        let derivative = reactor_vapor.scaled_by(flow7)
            - separator_vapor.scaled_by(flow8)
            - separator_vapor.scaled_by(flow9)
            - separator_liquid.scaled_by(flow10);

        offer::separator__state__vapor_a__derivative = derivative.component(0);
        offer::separator__state__vapor_b__derivative = derivative.component(1);
        offer::separator__state__vapor_c__derivative = derivative.component(2);
        offer::separator__state__liquid_d__derivative = derivative.component(3);
        offer::separator__state__liquid_e__derivative = derivative.component(4);
        offer::separator__state__liquid_f__derivative = derivative.component(5);
        offer::separator__state__liquid_g__derivative = derivative.component(6);
        offer::separator__state__liquid_h__derivative = derivative.component(7);
        offer::separator__state__enthalpy__derivative = enthalpy_reactor_outlet * flow7
            - need::flows__compressor_discharge_enthalpy * flow8
            - enthalpy_separator_vapor_uncorrected * flow9
            - enthalpy_separator_liquid * flow10
            + need::heat__separator_heat;
    }

    #[task]
    fn purge_analysis(&self) {
        offer::xmeas__stream9__component::<Mixture> = need::separator__vapor_composition::<Mixture>.scaled_by(100.0);
    }

    #[task]
    fn xmeas_readings(&self) {
        offer::xmeas__stream9__flow_rate = need::flows__stream_flow__9 * 0.359 / 35.3145;
        offer::xmeas__separator__temperature = need::separator__temperature;
        offer::xmeas__separator__level = (need::separator__liquid_volume - 27.5) / 290.0 * 100.0;
        offer::xmeas__separator__pressure = (need::separator__pressure - 760.0) / 760.0 * 101.325;
        offer::xmeas__stream10__flow_rate = need::flows__stream_flow__10 / need::separator__liquid_density / 35.3145;
        offer::xmeas__separator__cooling_water_outlet_temperature = need::heat__separator_cooling_water_return;
    }
}
