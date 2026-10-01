/* Documentação: docs/14-stripper.md */

use crate::physics::constants::{TepConstants, TEP_SPECIES};
use crate::units::feed::FEED_TEMPERATURE;
use monjolo::chemistry::{liquid_density, temperature_from_enthalpy, Mixture};

const STRIPPER_PRODUCT_RANGE: f64 = 1000.0;
const STRIPPER_STEAM_RANGE: f64 = 0.03;

#[monjolo::dynamic_model(tasks)]
pub struct Stripper {
    #[state]
    #[config(prefix = "state.stripper_liquid", components = ["A", "B", "C", "D", "E", "F", "G", "H"])]
    #[offer(prefix = "stripper.state", components = ["0", "1", "2", "3", "4", "5", "6", "7"])]
    liquid: [f64; 8],

    #[state]
    #[config(key = "state.stripper.energy")]
    #[offer(key = "stripper.state.8")]
    enthalpy: f64,

    constants: TepConstants,
}

#[monjolo::tasks(species = TEP_SPECIES, len = 8)]
impl Stripper {
    #[task]
    fn physical_state(&self) {
        let liquid = Mixture::new(self.liquid(), &TEP_SPECIES);
        let liquid_composition = liquid.mole_fractions();

        let specific_enthalpy = self.enthalpy() / liquid.total();
        let temperature = temperature_from_enthalpy(&liquid_composition.as_array(), need::separator__temperature, specific_enthalpy, 0, &self.constants);
        let density = liquid_density(&liquid_composition.as_array(), temperature, &self.constants);
        let volume_liquid = liquid.total() / density;

        offer::stripper__temperature = temperature;
        offer::stripper__liquid_volume = volume_liquid;
        offer::stripper__liquid_density = density;
        offer::stripper__liquid_composition::<Mixture> = liquid_composition;
    }

    #[task]
    fn product_flow(&self) {
        offer::flows__stream_flow__11 = need::valve__stripper_product__position * STRIPPER_PRODUCT_RANGE / 100.0;
    }

    #[task]
    fn flash_split(&self) {

        let ac_feed_flow = need::flows__stream_flow__4; // VAZÃO DO FEED A&C
        let underflow_flow = need::flows__stream_flow__10; // VAZÃO DO SEPARADOR
        let stripper_temperature = need::stripper__temperature; // TEMPERATURA PRÓPRIA
        let separator_liquid = need::separator__liquid_composition::<Mixture>; // COMPOSIÇÃO DO SEPARADOR
        let ac_feed_composition = need::flows__stream4_composition::<Mixture>; // COMPOSIÇÃO DO FEED A&C

        let flash_inlet = ac_feed_composition.scaled_by(ac_feed_flow) + separator_liquid.scaled_by(underflow_flow);

        let mut split_fraction = [0.995, 0.991, 0.990, 0.0, 0.0, 0.0, 0.0, 0.0];
        if underflow_flow > 0.1 {
            let temperature_factor = if stripper_temperature > 170.0 {
                stripper_temperature - 120.262
            } else if stripper_temperature < 5.292 {
                0.1
            } else {
                363.744 / (177.0 - stripper_temperature) - 2.22579488
            };
            let vapor_over_liquid = ac_feed_flow / underflow_flow * temperature_factor;
            split_fraction[3] = 8.5010 * vapor_over_liquid / (1.0 + 8.5010 * vapor_over_liquid);
            split_fraction[4] = 11.402 * vapor_over_liquid / (1.0 + 11.402 * vapor_over_liquid);
            split_fraction[5] = 11.795 * vapor_over_liquid / (1.0 + 11.795 * vapor_over_liquid);
            split_fraction[6] = 0.0480 * vapor_over_liquid / (1.0 + 0.0480 * vapor_over_liquid);
            split_fraction[7] = 0.0242 * vapor_over_liquid / (1.0 + 0.0242 * vapor_over_liquid);
        } else {
            split_fraction[3] = 0.9999;
            split_fraction[4] = 0.999;
            split_fraction[5] = 0.999;
            split_fraction[6] = 0.99;
            split_fraction[7] = 0.98;
        }

        let component_flow_5 = Mixture::new(split_fraction, &TEP_SPECIES) * flash_inlet;
        let component_flow_liquid = flash_inlet - component_flow_5;

        offer::flows__stream_flow__5 = component_flow_5.total();
        offer::flows__flash_vapor_component_flow::<Mixture> = component_flow_5;
        offer::flows__flash_liquid_component_flow::<Mixture> = component_flow_liquid;
    }

    #[task]
    fn heat_exchange(&self) {
        let condenser_ua = need::valve__stripper_steam__position * STRIPPER_STEAM_RANGE / 100.0;
        let stripper_temperature = need::stripper__temperature;
        offer::heat__condenser_heat = if stripper_temperature < 100.0 { condenser_ua * (100.0 - stripper_temperature) } else { 0.0 };
    }

    #[task]
    fn mass_and_energy_balance(&self) {
        let flash_liquid_flow = need::flows__flash_liquid_component_flow::<Mixture>;
        let flash_vapor_flow = need::flows__flash_vapor_component_flow::<Mixture>;
        let stripper_liquid = need::stripper__liquid_composition::<Mixture>;
        let stripper_temperature = need::stripper__temperature;
        let flow11 = need::flows__stream_flow__11;
        let flow4 = need::flows__stream_flow__4;
        let flow10 = need::flows__stream_flow__10;
        let flow5 = need::flows__stream_flow__5;
        let separator_temperature = need::separator__temperature;

        let enthalpy_feed_ac = need::flows__stream4_composition::<Mixture>.enthalpy(FEED_TEMPERATURE, 1, &self.constants);
        let enthalpy_separator_liquid = need::separator__liquid_composition::<Mixture>.enthalpy(separator_temperature, 0, &self.constants);
        let enthalpy_stripper_liquid = stripper_liquid.enthalpy(stripper_temperature, 0, &self.constants);

        let enthalpy_flash_vapor = flash_vapor_flow.mole_fractions().enthalpy(stripper_temperature, 1, &self.constants);

        let derivative = flash_liquid_flow - stripper_liquid.scaled_by(flow11);

        offer::stripper__state__0__derivative = derivative.component(0);
        offer::stripper__state__1__derivative = derivative.component(1);
        offer::stripper__state__2__derivative = derivative.component(2);
        offer::stripper__state__3__derivative = derivative.component(3);
        offer::stripper__state__4__derivative = derivative.component(4);
        offer::stripper__state__5__derivative = derivative.component(5);
        offer::stripper__state__6__derivative = derivative.component(6);
        offer::stripper__state__7__derivative = derivative.component(7);
        offer::stripper__state__8__derivative = enthalpy_feed_ac * flow4 + enthalpy_separator_liquid * flow10
            - enthalpy_flash_vapor * flow5
            - enthalpy_stripper_liquid * flow11
            + need::heat__condenser_heat;
    }

    #[task]
    fn product_analysis(&self) {
        let composition = need::stripper__liquid_composition::<Mixture>;
        offer::xmeas__stream11__component__d = composition.component(3) * 100.0;
        offer::xmeas__stream11__component__e = composition.component(4) * 100.0;
        offer::xmeas__stream11__component__f = composition.component(5) * 100.0;
        offer::xmeas__stream11__component__g = composition.component(6) * 100.0;
        offer::xmeas__stream11__component__h = composition.component(7) * 100.0;
    }

    #[task]
    fn xmeas_readings(&self) {
        offer::xmeas__stripper__level = (need::stripper__liquid_volume - 78.25) / 156.5 * 100.0;
        offer::xmeas__stream11__flow_rate = need::flows__stream_flow__11 / need::stripper__liquid_density / 35.3145;
        offer::xmeas__stripper__temperature = need::stripper__temperature;
        offer::xmeas__stripper__steam_flow_rate = need::heat__condenser_heat * 1.04e3 * 0.454;
    }
}
