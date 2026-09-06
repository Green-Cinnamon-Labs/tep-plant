/* tep/dynamics/separator.rs */

use crate::physics::constants::TepConstants;
use monjolo::chemistry::{liquid_density, mixture_enthalpy, temperature_from_enthalpy};

const SEPARATOR_VOLUME: f64 = 3500.0; /* volume total do separador vapor/líquido [m³] */
const GAS_CONSTANT: f64 = 998.9; /* R em [mmHg·m³/(kmol·K)] */
const SEPARATOR_UNDERFLOW_RANGE: f64 = 1500.0; /* VRNG (TEINIT) da válvula de underflow */
/* Nominal — mesmo `s_zero` de TepDisturbanceState, canal 5 (TCWS, "condenser cooling water temp"). */
const SEPARATOR_COOLING_WATER_RETURN: f64 = 40.0;

/** Terceira unidade migrada pro scheduler de dataflow topológico (issue 10), depois de Feed e
Compressor. Absorve de `flows.rs`: Block 22/25 (slots 9/10, purge e underflow). De `heat.rs`:
Block 33 (troca térmica do separador). De `derivatives.rs`: a seção "Separador" do balanço de
massa/energia (Block 40, YP(10..18)). De `purge_analyzer.rs`: XMEAS 29-36 (Purge Gas Analysis).
*/
#[monjolo::dynamic_model(tasks)]
pub struct Separator {
    /* Estado próprio (9 números) — mesmo split de Reactor, mesmo motivo (chave de config não
    uniforme entre vapor/líquido e entalpia).
    */
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

#[monjolo::tasks]
impl Separator {
    /* Bloco 1: balanço de energia próprio → temperatura/pressão/composição/volume/densidade —
    igual ao `compute()` monolítico de antes, agora uma tarefa entre várias.
    */
    #[need(key = "reactor.temperature")]
    #[offer(key = "separator.temperature")]
    #[offer(key = "separator.pressure")]
    #[offer(key = "separator.liquid_volume")]
    #[offer(key = "separator.liquid_density")]
    #[offer(key = "separator.total_vapor_kmol")]
    #[offer(prefix = "separator.liquid_composition", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    #[offer(prefix = "separator.vapor_composition", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    #[allow(clippy::too_many_arguments)]
    fn thermodynamics(&self, reactor_temperature: f64) -> (f64, f64, f64, f64, f64, [f64; 8], [f64; 8]) {
        let vapor_group = self.vapor();
        let liquid_group = self.liquid();
        let mut vapor_moles = [0.0f64; 8];
        let mut liquid_moles = [0.0f64; 8];
        for i in 0..3 {
            vapor_moles[i] = vapor_group[i];
        }
        for i in 3..8 {
            liquid_moles[i] = liquid_group[i - 3];
        }
        let total_enthalpy = self.enthalpy();

        let total_liquid_moles: f64 = liquid_moles.iter().sum();
        let mut liquid_composition = [0.0f64; 8];
        for i in 0..8 {
            liquid_composition[i] = liquid_moles[i] / total_liquid_moles;
        }

        let specific_enthalpy = total_enthalpy / total_liquid_moles;
        let temperature = temperature_from_enthalpy(&liquid_composition, reactor_temperature, specific_enthalpy, 0, &self.constants);
        let temperature_k = temperature + 273.15;
        let density = liquid_density(&liquid_composition, temperature, &self.constants);
        let volume_liquid = total_liquid_moles / density;
        let volume_vapor = SEPARATOR_VOLUME - volume_liquid;

        let mut partial_pressures = [0.0f64; 8];
        let mut pressure = 0.0f64;
        for i in 0..3 {
            partial_pressures[i] = vapor_moles[i] * GAS_CONSTANT * temperature_k / volume_vapor;
            pressure += partial_pressures[i];
        }
        for i in 3..8 {
            partial_pressures[i] = (self.constants.avp[i] + self.constants.bvp[i] / (temperature + self.constants.cvp[i])).exp() * liquid_composition[i];
            pressure += partial_pressures[i];
        }

        let mut vapor_composition = [0.0f64; 8];
        for i in 0..8 {
            vapor_composition[i] = partial_pressures[i] / pressure;
        }
        let total_vapor_moles = pressure * volume_vapor / GAS_CONSTANT / temperature_k;

        (temperature, pressure, volume_liquid, density, total_vapor_moles, liquid_composition, vapor_composition)
    }

    /* Bloco 2 (ex-Flows, Block 22/25): purge (slot 9, dependente de pressão+composição próprias) e
    underflow (slot 10, puramente linear na válvula — sem acoplamento nenhum, mas fica junto por
    ser a outra saída direta do vaso).
    */
    #[need(key = "separator.pressure")]
    #[need(prefix = "separator.vapor_composition", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    #[need(key = "valve.purge.position")]
    #[need(key = "valve.separator_underflow.position")]
    #[offer(key = "flows.stream_flow.9")]
    #[offer(key = "flows.stream_flow.10")]
    fn outlet_flows(&self, separator_pressure: f64, separator_vapor: [f64; 8], purge_position: f64, underflow_position: f64) -> (f64, f64) {
        let mol_weight = |z: &[f64; 8]| -> f64 { (0..8).map(|i| z[i] * self.constants.xmw[i]).sum() };
        let purge_flow = purge_position * 0.151169 * (separator_pressure - 760.0).max(0.0).sqrt() / mol_weight(&separator_vapor);
        let underflow_flow = underflow_position * SEPARATOR_UNDERFLOW_RANGE / 100.0;

        (purge_flow, underflow_flow)
    }

    /* Bloco 3 (ex-Heat, Block 33): troca térmica no separador — UAS depende da vazão reator→
    separador; a temperatura de referência é a do REATOR (não do separador — TST(8) aponta pro
    reator no teprob.f, Block 20), preservado por fidelidade.
    */
    #[need(key = "reactor.temperature")]
    #[need(key = "flows.stream_flow.7")]
    #[offer(key = "heat.separator_heat")]
    #[offer(key = "heat.separator_cooling_water_return")]
    fn heat(&self, reactor_temperature: f64, reactor_to_separator_flow: f64) -> (f64, f64) {
        let uas = 0.404655 * (1.0 - 1.0 / (1.0 + (reactor_to_separator_flow / 3528.73).powi(4)));
        let separator_heat = uas * (SEPARATOR_COOLING_WATER_RETURN - reactor_temperature) * (1.0 - 0.25 * 0.0);

        (separator_heat, SEPARATOR_COOLING_WATER_RETURN)
    }

    /* Bloco 4 (ex-Derivatives, Block 40 YP(10..18)): balanço de massa/energia do próprio estado.
    `enthalpy_separator_liquid` é recomputada aqui (não lida de volta) — mesmo padrão já usado em
    `derivatives.rs` pras entalpias de feed: quem precisa recalcula fresco a partir de composição+
    temperatura já publicadas, em vez de depender de mais uma chave.
    */
    #[need(prefix = "reactor.vapor_composition", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    #[need(key = "reactor.temperature")]
    #[need(key = "flows.stream_flow.7")]
    #[need(key = "flows.stream_flow.8")]
    #[need(key = "flows.stream_flow.9")]
    #[need(key = "flows.stream_flow.10")]
    #[need(prefix = "separator.vapor_composition", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    #[need(prefix = "separator.liquid_composition", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    #[need(key = "separator.temperature")]
    #[need(key = "flows.compressor_discharge_enthalpy")]
    #[need(key = "heat.separator_heat")]
    #[offer(prefix = "separator.state", components = ["vapor_a.derivative", "vapor_b.derivative", "vapor_c.derivative"])]
    #[offer(prefix = "separator.state", components = ["liquid_d.derivative", "liquid_e.derivative", "liquid_f.derivative", "liquid_g.derivative", "liquid_h.derivative"])]
    #[offer(key = "separator.state.enthalpy.derivative")]
    #[allow(clippy::too_many_arguments)]
    fn yp_derivative(
        &self,
        reactor_vapor: [f64; 8],
        reactor_temperature: f64,
        flow7: f64,
        flow8: f64,
        flow9: f64,
        flow10: f64,
        separator_vapor: [f64; 8],
        separator_liquid: [f64; 8],
        separator_temperature: f64,
        compressor_discharge_enthalpy: f64,
        separator_heat: f64,
    ) -> ([f64; 3], [f64; 5], f64) {
        let enthalpy_reactor_outlet = mixture_enthalpy(&reactor_vapor, reactor_temperature, 1, &self.constants);
        /* SEM a correção de Block 24 (compressor) — é o que HST(10) preserva no original, por ter
        sido copiado ANTES da correção rodar.
        */
        let enthalpy_separator_vapor_uncorrected = mixture_enthalpy(&separator_vapor, separator_temperature, 1, &self.constants);
        let enthalpy_separator_liquid = mixture_enthalpy(&separator_liquid, separator_temperature, 0, &self.constants);

        let mut vapor_derivative = [0.0f64; 3];
        let mut liquid_derivative = [0.0f64; 5];
        for i in 0..8 {
            let value = reactor_vapor[i] * flow7 - separator_vapor[i] * flow8 - separator_vapor[i] * flow9 - separator_liquid[i] * flow10;
            if i < 3 {
                vapor_derivative[i] = value;
            } else {
                liquid_derivative[i - 3] = value;
            }
        }
        let enthalpy_derivative = enthalpy_reactor_outlet * flow7
            - compressor_discharge_enthalpy * flow8
            - enthalpy_separator_vapor_uncorrected * flow9
            - enthalpy_separator_liquid * flow10
            + separator_heat;

        (vapor_derivative, liquid_derivative, enthalpy_derivative)
    }

    /* Bloco 5 (ex-purge_analyzer.rs): XMEAS 29-36, Purge Gas Analysis (Stream 9) — a mesma
    composição de vapor que alimenta o recycle na stream 8 (Block 27 de teprob.f, `FCM(I,9)`/
    `FCM(I,8)` usam o mesmo `XST(.,9)=XST(.,8)`), convertida de fração molar pra mol%.
    */
    #[need(prefix = "separator.vapor_composition", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    #[offer(prefix = "xmeas.stream9.component", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    fn purge_analysis(&self, composition: [f64; 8]) -> [f64; 8] {
        let mut mole_percent = [0.0f64; 8];
        for i in 0..8 {
            mole_percent[i] = composition[i] * 100.0;
        }
        mole_percent
    }

    /* Bloco 6 (ex-measured.rs, Block 35): XMEAS 10 (Purge Rate, stream9), 11-13 (temperatura/
    nível/pressão do separador), 14 (Separator Underflow, stream10), 22 (temperatura de saída da
    água de resfriamento) — conversões preservadas exatamente do original.
    */
    #[need(key = "flows.stream_flow.9")]
    #[need(key = "separator.temperature")]
    #[need(key = "separator.liquid_volume")]
    #[need(key = "separator.pressure")]
    #[need(key = "flows.stream_flow.10")]
    #[need(key = "separator.liquid_density")]
    #[need(key = "heat.separator_cooling_water_return")]
    #[offer(key = "xmeas.stream9.flow_rate")]
    #[offer(key = "xmeas.separator.temperature")]
    #[offer(key = "xmeas.separator.level")]
    #[offer(key = "xmeas.separator.pressure")]
    #[offer(key = "xmeas.stream10.flow_rate")]
    #[offer(key = "xmeas.separator.cooling_water_outlet_temperature")]
    #[allow(clippy::too_many_arguments)]
    fn xmeas_conversions(
        &self,
        purge_flow: f64,
        temperature: f64,
        liquid_volume: f64,
        pressure: f64,
        underflow_flow: f64,
        liquid_density: f64,
        cooling_water_return: f64,
    ) -> (f64, f64, f64, f64, f64, f64) {
        let xmeas_purge_rate = purge_flow * 0.359 / 35.3145;
        let xmeas_level = (liquid_volume - 27.5) / 290.0 * 100.0;
        let xmeas_pressure = (pressure - 760.0) / 760.0 * 101.325;
        let xmeas_underflow = underflow_flow / liquid_density / 35.3145;

        (xmeas_purge_rate, temperature, xmeas_level, xmeas_pressure, xmeas_underflow, cooling_water_return)
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
            ("state.separator_vapor.A", 1.0),
            ("state.separator_vapor.B", 2.0),
            ("state.separator_vapor.C", 3.0),
            ("state.separator_vapor.D", 4.0),
            ("state.separator_vapor.E", 5.0),
            ("state.separator_vapor.F", 6.0),
            ("state.separator_vapor.G", 7.0),
            ("state.separator_vapor.H", 8.0),
            ("state.separator.energy", 42.0),
        ]);

        let separator = Separator::new(&mut registry.borrow_mut(), &initial);

        assert_eq!(separator.vapor(), [1.0, 2.0, 3.0]);
        assert_eq!(separator.liquid(), [4.0, 5.0, 6.0, 7.0, 8.0]);
        assert_eq!(separator.enthalpy(), 42.0);
    }

    #[test]
    fn new_defaults_missing_keys_to_zero() {
        let registry = StateRegistry::shared();
        let initial = Snapshot::from_pairs(&[]);

        let separator = Separator::new(&mut registry.borrow_mut(), &initial);

        assert_eq!(separator.vapor(), [0.0; 3]);
        assert_eq!(separator.liquid(), [0.0; 5]);
        assert_eq!(separator.enthalpy(), 0.0);
    }
}
