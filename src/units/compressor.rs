/* Documentação: docs/15-compressor.md */

use crate::physics::constants::{TepConstants, TEP_SPECIES};
use crate::units::feed::{FEED_A_COMPOSITION, FEED_D_COMPOSITION, FEED_E_COMPOSITION, FEED_TEMPERATURE};
use monjolo::chemistry::{temperature_from_enthalpy, Mixture, GAS_CONSTANT};

const COMPRESSOR_VESSEL_VOLUME: f64 = 5000.0;
const COMPRESSOR_FLOW_MAX: f64 = 280275.0;
const COMPRESSOR_PRESSURE_RATIO_MAX: f64 = 1.3;

#[monjolo::dynamic_model(tasks)]
pub struct Compressor {
    #[state]
    #[config(prefix = "state.compressor_vapor", components = ["A", "B", "C", "D", "E", "F", "G", "H"])]
    #[offer(prefix = "compressor.state", components = ["0", "1", "2", "3", "4", "5", "6", "7"])]
    vapor: [f64; 8],

    #[state]
    #[config(key = "state.compressor.energy")]
    #[offer(key = "compressor.state.8")]
    enthalpy: f64,

    constants: TepConstants,
}

#[monjolo::tasks(species = TEP_SPECIES, len = 8)]
impl Compressor {
    #[task]
    fn physical_state(&self) {
        let vapor = Mixture::new(self.vapor(), &TEP_SPECIES);
        let vapor_composition = vapor.mole_fractions();
        let total_vapor_moles = vapor.total();

        let specific_enthalpy = self.enthalpy() / total_vapor_moles;
        let temperature = temperature_from_enthalpy(&vapor_composition.as_array(), need::separator__temperature, specific_enthalpy, 2, &self.constants);
        let temperature_k = temperature + 273.15;
        let pressure = total_vapor_moles * GAS_CONSTANT * temperature_k / COMPRESSOR_VESSEL_VOLUME;

        offer::compressor__temperature = temperature;
        offer::compressor__pressure = pressure;
        offer::compressor__vapor_composition::<Mixture> = vapor_composition;
    }

    #[task]
    fn outlet_flows(&self) {
        let compressor_pressure = need::compressor__pressure;
        let separator_pressure = need::separator__pressure;
        let separator_temperature = need::separator__temperature;
        let separator_vapor = need::separator__vapor_composition::<Mixture>;
        let compressor_vapor = need::compressor__vapor_composition::<Mixture>;

        let mw6 = compressor_vapor.dot(&self.constants.xmw);
        let mw8 = separator_vapor.dot(&self.constants.xmw);

        let flow6 = 1937.6 * (compressor_pressure - need::reactor__pressure).max(0.0).sqrt() / mw6;

        let pressure_ratio = (compressor_pressure / separator_pressure).max(1.0).min(COMPRESSOR_PRESSURE_RATIO_MAX);
        let flow_coeff = COMPRESSOR_FLOW_MAX / 1.197;
        let mut compressor_mass_flow = COMPRESSOR_FLOW_MAX + flow_coeff * (1.0 - pressure_ratio.powi(3));
        let compressor_work = compressor_mass_flow * (separator_temperature + 273.15) * 1.8e-6 * 1.9872
            * (compressor_pressure - separator_pressure)
            / (mw8 * separator_pressure);
        compressor_mass_flow -= need::valve__compressor_recycle__position * 53.349 * (compressor_pressure - separator_pressure).max(0.0).sqrt();
        compressor_mass_flow = compressor_mass_flow.max(1e-3);
        let flow8 = compressor_mass_flow / mw8;

        let separator_vapor_enthalpy = separator_vapor.enthalpy(separator_temperature, 1, &self.constants);
        let compressor_discharge_enthalpy = separator_vapor_enthalpy + compressor_work / flow8;

        offer::flows__stream_flow__6 = flow6;
        offer::flows__stream_flow__8 = flow8;
        offer::flows__compressor_work = compressor_work;
        offer::flows__compressor_discharge_enthalpy = compressor_discharge_enthalpy;
    }

    #[task]
    fn mass_and_energy_balance(&self) {
        let flow2 = need::flows__stream_flow__2; // FEED D
        let flow3 = need::flows__stream_flow__3; // FEED E
        let flow1 = need::flows__stream_flow__1; // FEED A
        let flow5 = need::flows__stream_flow__5; // VAPOR DO STRIPPER
        let flow6 = need::flows__stream_flow__6; // RECICLO PRO REATOR
        let flow8 = need::flows__stream_flow__8; // RECICLO DO SEPARADOR
        let flash_vapor_flow = need::flows__flash_vapor_component_flow::<Mixture>; // COMPOSIÇÃO DO STRIPPER
        let separator_vapor = need::separator__vapor_composition::<Mixture>; // COMPOSIÇÃO DO SEPARADOR
        let compressor_vapor = need::compressor__vapor_composition::<Mixture>; // COMPOSIÇÃO PRÓPRIA

        let enthalpy_flash_vapor = flash_vapor_flow.mole_fractions().enthalpy(need::stripper__temperature, 1, &self.constants);
        let enthalpy_feed_d = Mixture::new(FEED_D_COMPOSITION, &TEP_SPECIES).enthalpy(need::flows__d_feed_temperature, 1, &self.constants);
        let enthalpy_feed_e = Mixture::new(FEED_E_COMPOSITION, &TEP_SPECIES).enthalpy(FEED_TEMPERATURE, 1, &self.constants);
        let enthalpy_feed_a = Mixture::new(FEED_A_COMPOSITION, &TEP_SPECIES).enthalpy(FEED_TEMPERATURE, 1, &self.constants);
        let enthalpy_compressor_recycle = compressor_vapor.enthalpy(need::compressor__temperature, 1, &self.constants);

        let derivative = Mixture::new(FEED_D_COMPOSITION, &TEP_SPECIES).scaled_by(flow2)
            + Mixture::new(FEED_E_COMPOSITION, &TEP_SPECIES).scaled_by(flow3)
            + Mixture::new(FEED_A_COMPOSITION, &TEP_SPECIES).scaled_by(flow1)
            + flash_vapor_flow
            + separator_vapor.scaled_by(flow8)
            - compressor_vapor.scaled_by(flow6);

        offer::compressor__state__0__derivative = derivative.component(0);
        offer::compressor__state__1__derivative = derivative.component(1);
        offer::compressor__state__2__derivative = derivative.component(2);
        offer::compressor__state__3__derivative = derivative.component(3);
        offer::compressor__state__4__derivative = derivative.component(4);
        offer::compressor__state__5__derivative = derivative.component(5);
        offer::compressor__state__6__derivative = derivative.component(6);
        offer::compressor__state__7__derivative = derivative.component(7);
        offer::compressor__state__8__derivative = enthalpy_feed_d * flow2
            + enthalpy_feed_e * flow3
            + enthalpy_feed_a * flow1
            + enthalpy_flash_vapor * flow5
            + need::flows__compressor_discharge_enthalpy * flow8
            - enthalpy_compressor_recycle * flow6;

    }

    #[task]
    fn reactor_feed_analysis(&self) {
        let composition = need::compressor__vapor_composition::<Mixture>;
        offer::xmeas__stream6__component__a = composition.component(0) * 100.0;
        offer::xmeas__stream6__component__b = composition.component(1) * 100.0;
        offer::xmeas__stream6__component__c = composition.component(2) * 100.0;
        offer::xmeas__stream6__component__d = composition.component(3) * 100.0;
        offer::xmeas__stream6__component__e = composition.component(4) * 100.0;
        offer::xmeas__stream6__component__f = composition.component(5) * 100.0;
    }

    #[task]
    fn xmeas_readings(&self) {
        offer::xmeas__stream8__flow_rate = need::flows__stream_flow__8 * 0.359 / 35.3145;
        offer::xmeas__stream6__flow_rate = need::flows__stream_flow__6 * 0.359 / 35.3145;
        offer::xmeas__stripper__pressure = (need::compressor__pressure - 760.0) / 760.0 * 101.325;
        offer::xmeas__compressor__work = need::flows__compressor_work * 0.29307e3;
    }
}
