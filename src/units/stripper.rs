/* tep/units/stripper.rs */

use crate::units::feed::{FEED_AC_COMPOSITION, FEED_TEMPERATURE};
use crate::physics::constants::TepConstants;
use monjolo::chemistry::{liquid_density, mixture_enthalpy, temperature_from_enthalpy};

const STRIPPER_PRODUCT_RANGE: f64 = 1000.0; /* VRNG (TEINIT) da válvula de produto */
const STRIPPER_STEAM_RANGE: f64 = 0.03; /* VRNG (TEINIT) da válvula de vapor (UAC) */

/** Quarta unidade migrada pro scheduler de dataflow topológico (issue 10), depois de Feed/
Compressor/Separator. Absorve de `flows.rs`: Block 22 (slot 12, produto), Block 25-28 (flash split
completo — entrada combinada A&C feed + underflow do separador, fração de split, slots 4/11 de
saída). De `heat.rs`: Block 34 (condenser/reboiler) + o próprio `condenser_ua` (Block 22, UAC —
nunca tinha dono, só usado aqui mesmo). De `derivatives.rs`: a seção "Stripper" do balanço de
massa/energia (Block 40, YP(19..27)). De `product_analyzer.rs`: XMEAS 37-41 (Product Analysis).
*/
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

#[monjolo::tasks]
impl Stripper {
    /* Bloco 1: balanço de energia próprio → temperatura/volume/densidade/composição — igual ao
    `compute()` monolítico de antes.
    */
    #[need(key = "separator.temperature")]
    #[offer(key = "stripper.temperature")]
    #[offer(key = "stripper.liquid_volume")]
    #[offer(key = "stripper.liquid_density")]
    #[offer(prefix = "stripper.liquid_composition", components = ["0", "1", "2", "3", "4", "5", "6", "7"])]
    fn thermodynamics(&self, separator_temperature: f64) -> (f64, f64, f64, [f64; 8]) {
        let liquid_group = self.liquid();
        let mut liquid_moles = [0.0f64; 8];
        for i in 0..8 {
            liquid_moles[i] = liquid_group[i];
        }
        let total_enthalpy = self.enthalpy();

        let total_liquid_moles: f64 = liquid_moles.iter().sum();
        let mut liquid_composition = [0.0f64; 8];
        for i in 0..8 {
            liquid_composition[i] = liquid_moles[i] / total_liquid_moles;
        }

        let specific_enthalpy = total_enthalpy / total_liquid_moles;
        let temperature = temperature_from_enthalpy(&liquid_composition, separator_temperature, specific_enthalpy, 0, &self.constants);
        let density = liquid_density(&liquid_composition, temperature, &self.constants);
        let volume_liquid = total_liquid_moles / density;

        (temperature, volume_liquid, density, liquid_composition)
    }

    /* Bloco 2 (ex-Flows, Block 22 slot 12): produto do stripper — puramente linear na válvula,
    sem acoplamento nenhum, mesmo padrão do underflow do Separator.
    */
    #[need(key = "valve.stripper_product.position")]
    #[offer(key = "flows.stream_flow.12")]
    fn outlet_flow(&self, position: f64) -> f64 {
        position * STRIPPER_PRODUCT_RANGE / 100.0
    }

    /* Bloco 3 (ex-Flows, Blocks 25-28): split flash da entrada combinada (A&C feed direto do Feed +
    underflow do separador). Fração de split fixa (SFR, TEINIT) pros componentes A/B/C — nunca
    recalculada; D-H dependem da própria temperatura (Block 26) — ver comentário original de
    `flows.rs` sobre o bug real encontrado aqui (A&C caindo inteiro no líquido, inundando o
    stripper) — preservado tal qual.
    */
    #[need(key = "flows.stream_flow.3")]
    #[need(key = "flows.stream_flow.10")]
    #[need(prefix = "separator.liquid_composition", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    #[need(key = "stripper.temperature")]
    #[offer(key = "flows.stream_flow.4")]
    #[offer(key = "flows.stream_flow.11")]
    #[offer(prefix = "flows.flash_vapor_component_flow", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    #[offer(prefix = "flows.flash_liquid_component_flow", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    fn flash_split(&self, ac_feed_flow: f64, underflow_flow: f64, separator_liquid: [f64; 8], stripper_temperature: f64) -> (f64, f64, [f64; 8], [f64; 8]) {
        let mut component_flow_3 = [0.0f64; 8];
        let mut component_flow_10 = [0.0f64; 8];
        for i in 0..8 {
            component_flow_3[i] = FEED_AC_COMPOSITION[i] * ac_feed_flow;
            component_flow_10[i] = separator_liquid[i] * underflow_flow;
        }

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

        let mut flash_inlet = [0.0f64; 8];
        for i in 0..8 {
            flash_inlet[i] = component_flow_3[i] + component_flow_10[i];
        }

        let mut component_flow_4 = [0.0f64; 8];
        let mut component_flow_11 = [0.0f64; 8];
        let mut flow4 = 0.0f64;
        let mut flow11 = 0.0f64;
        for i in 0..8 {
            component_flow_4[i] = split_fraction[i] * flash_inlet[i];
            component_flow_11[i] = flash_inlet[i] - component_flow_4[i];
            flow4 += component_flow_4[i];
            flow11 += component_flow_11[i];
        }

        (flow4, flow11, component_flow_4, component_flow_11)
    }

    /* Bloco 4 (ex-Heat, Block 34 + o UAC de Block 22 — que nunca teve dono próprio além de ser
    consumido aqui mesmo): resfriamento condicional do reboiler — só troca calor se a temperatura
    do stripper estiver abaixo de 100°C.
    */
    #[need(key = "valve.stripper_steam.position")]
    #[need(key = "stripper.temperature")]
    #[offer(key = "heat.condenser_heat")]
    fn heat(&self, steam_position: f64, stripper_temperature: f64) -> f64 {
        let condenser_ua = steam_position * STRIPPER_STEAM_RANGE / 100.0;
        if stripper_temperature < 100.0 {
            condenser_ua * (100.0 - stripper_temperature)
        } else {
            0.0
        }
    }

    /* Bloco 5 (ex-Derivatives, Block 40 YP(19..27)): balanço de massa/energia do próprio estado.
    Entalpias recomputadas frescas (mesmo padrão já usado nas outras unidades) — nada aqui lê uma
    entalpia publicada por outro componente.
    */
    #[need(prefix = "flows.flash_liquid_component_flow", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    #[need(prefix = "flows.flash_vapor_component_flow", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    #[need(prefix = "stripper.liquid_composition", components = ["0", "1", "2", "3", "4", "5", "6", "7"])]
    #[need(key = "stripper.temperature")]
    #[need(key = "flows.stream_flow.12")]
    #[need(key = "flows.stream_flow.3")]
    #[need(key = "flows.stream_flow.10")]
    #[need(key = "flows.stream_flow.4")]
    #[need(prefix = "separator.liquid_composition", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    #[need(key = "separator.temperature")]
    #[need(key = "heat.condenser_heat")]
    #[offer(prefix = "stripper.state", components = ["0.derivative", "1.derivative", "2.derivative", "3.derivative", "4.derivative", "5.derivative", "6.derivative", "7.derivative"])]
    #[offer(key = "stripper.state.8.derivative")]
    #[allow(clippy::too_many_arguments)]
    fn yp_derivative(
        &self,
        flash_liquid_flow: [f64; 8],
        flash_vapor_flow: [f64; 8],
        stripper_liquid: [f64; 8],
        stripper_temperature: f64,
        flow12: f64,
        flow3: f64,
        flow10: f64,
        flow4: f64,
        separator_liquid: [f64; 8],
        separator_temperature: f64,
        condenser_heat: f64,
    ) -> ([f64; 8], f64) {
        let enthalpy_feed_ac = mixture_enthalpy(&FEED_AC_COMPOSITION, FEED_TEMPERATURE, 1, &self.constants);
        let enthalpy_separator_liquid = mixture_enthalpy(&separator_liquid, separator_temperature, 0, &self.constants);
        let enthalpy_stripper_liquid = mixture_enthalpy(&stripper_liquid, stripper_temperature, 0, &self.constants);

        let flash_vapor_total: f64 = flash_vapor_flow.iter().sum();
        let mut flash_vapor_composition = [0.0f64; 8];
        if flash_vapor_total > 0.0 {
            for i in 0..8 {
                flash_vapor_composition[i] = flash_vapor_flow[i] / flash_vapor_total;
            }
        }
        let enthalpy_flash_vapor = mixture_enthalpy(&flash_vapor_composition, stripper_temperature, 1, &self.constants);

        let mut liquid_derivative = [0.0f64; 8];
        for i in 0..8 {
            liquid_derivative[i] = flash_liquid_flow[i] - stripper_liquid[i] * flow12;
        }
        let enthalpy_derivative = enthalpy_feed_ac * flow3 + enthalpy_separator_liquid * flow10
            - enthalpy_flash_vapor * flow4
            - enthalpy_stripper_liquid * flow12
            + condenser_heat;

        (liquid_derivative, enthalpy_derivative)
    }

    /* Bloco 6 (ex-product_analyzer.rs): XMEAS 37-41, Product Analysis (Stream 11) — a composição
    líquida própria (a mesma que `valve.stripper_product.position` escoa) convertida pra mol%.
    */
    #[need(prefix = "stripper.liquid_composition", components = ["3", "4", "5", "6", "7"])]
    #[offer(prefix = "xmeas.stream11.component", components = ["d", "e", "f", "g", "h"])]
    fn product_analysis(&self, composition: [f64; 5]) -> [f64; 5] {
        let mut mole_percent = [0.0f64; 5];
        for i in 0..5 {
            mole_percent[i] = composition[i] * 100.0;
        }
        mole_percent
    }

    /* Bloco 7 (ex-measured.rs, Block 35): XMEAS 15 (Stripper Level), 17 (Stripper Underflow,
    stream11), 18 (Stripper Temperature), 19 (Stripper Steam Flow) — conversões preservadas
    exatamente do original. VTC = 156.5 (TEINIT) pro nível.
    */
    #[need(key = "stripper.liquid_volume")]
    #[need(key = "flows.stream_flow.12")]
    #[need(key = "stripper.liquid_density")]
    #[need(key = "stripper.temperature")]
    #[need(key = "heat.condenser_heat")]
    #[offer(key = "xmeas.stripper.level")]
    #[offer(key = "xmeas.stream11.flow_rate")]
    #[offer(key = "xmeas.stripper.temperature")]
    #[offer(key = "xmeas.stripper.steam_flow_rate")]
    fn xmeas_conversions(&self, liquid_volume: f64, product_flow: f64, liquid_density: f64, temperature: f64, condenser_heat: f64) -> (f64, f64, f64, f64) {
        let xmeas_level = (liquid_volume - 78.25) / 156.5 * 100.0;
        let xmeas_underflow = product_flow / liquid_density / 35.3145;
        let xmeas_steam_flow = condenser_heat * 1.04e3 * 0.454;

        (xmeas_level, xmeas_underflow, temperature, xmeas_steam_flow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use monjolo::snapshot::Snapshot;
    use monjolo::state_registry::StateRegistry;

    #[test]
    fn new_seeds_own_state_with_initial_condition() {
        let registry = StateRegistry::shared();
        let initial = Snapshot::from_pairs(&[
            ("state.stripper_liquid.A", 1.0),
            ("state.stripper_liquid.B", 2.0),
            ("state.stripper_liquid.C", 3.0),
            ("state.stripper_liquid.D", 4.0),
            ("state.stripper_liquid.E", 5.0),
            ("state.stripper_liquid.F", 6.0),
            ("state.stripper_liquid.G", 7.0),
            ("state.stripper_liquid.H", 8.0),
            ("state.stripper.energy", 42.0),
        ]);

        let stripper = Stripper::new(&mut registry.borrow_mut(), &initial);

        assert_eq!(stripper.liquid(), [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
        assert_eq!(stripper.enthalpy(), 42.0);
    }

    #[test]
    fn new_defaults_missing_keys_to_zero() {
        let registry = StateRegistry::shared();
        let initial = Snapshot::from_pairs(&[]);

        let stripper = Stripper::new(&mut registry.borrow_mut(), &initial);

        assert_eq!(stripper.liquid(), [0.0; 8]);
        assert_eq!(stripper.enthalpy(), 0.0);
    }
}
