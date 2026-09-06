/* tep/dynamics/compressor.rs */

use crate::dynamics::feed::{FEED_A_COMPOSITION, FEED_D_COMPOSITION, FEED_E_COMPOSITION, FEED_TEMPERATURE};
use crate::physics::constants::TepConstants;
use monjolo::chemistry::{mixture_enthalpy, temperature_from_enthalpy};

const COMPRESSOR_VESSEL_VOLUME: f64 = 5000.0; /* volume do vaso do compressor/condensador [m³] */
const GAS_CONSTANT: f64 = 998.9; /* R em [mmHg·m³/(kmol·K)] */
const COMPRESSOR_FLOW_MAX: f64 = 280275.0; /* vazão mássica máxima do compressor [kg/h] */
const COMPRESSOR_PRESSURE_RATIO_MAX: f64 = 1.3;

/** Segunda unidade migrada pro scheduler de dataflow topológico (issue 10), depois de `Feed` —
primeira com `#[state]` própria E várias tarefas cruzando fronteira de outras unidades. Absorve de
`flows.rs`: Block 23 (slot 5, recycle compressor→reator), Block 24 (curva característica + anti-
surge, slot 8 — separador→compressor, mas é a curva do PRÓPRIO compressor que decide quanto ele
puxa, daí o dono ser este arquivo), Block 31 (slot 6, bypass = cópia do slot 5). Absorve de
`derivatives.rs`: a seção "Compressor" do balanço de massa/energia (Block 40, YP(28..36)).
*/
#[monjolo::dynamic_model(after = ["Stripper"], tasks)]
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

#[monjolo::tasks]
impl Compressor {
    /* Bloco 1: balanço de energia próprio → temperatura/pressão/composição — igual ao `compute()`
    monolítico de antes, só que agora é UMA tarefa entre várias, com seu próprio `needs`/`offers`.
    */
    #[need(key = "separator.temperature")]
    #[offer(key = "compressor.temperature")]
    #[offer(key = "compressor.pressure")]
    #[offer(prefix = "compressor.vapor_composition", components = ["0", "1", "2", "3", "4", "5", "6", "7"])]
    fn thermodynamics(&self, separator_temperature: f64) -> (f64, f64, [f64; 8]) {
        let vapor_group = self.vapor();
        let mut vapor_moles = [0.0f64; 8];
        for i in 0..8 {
            vapor_moles[i] = vapor_group[i];
        }
        let total_enthalpy = self.enthalpy();

        let total_vapor_moles: f64 = vapor_moles.iter().sum();
        let mut vapor_composition = [0.0f64; 8];
        for i in 0..8 {
            vapor_composition[i] = vapor_moles[i] / total_vapor_moles;
        }

        let specific_enthalpy = total_enthalpy / total_vapor_moles;
        let temperature = temperature_from_enthalpy(&vapor_composition, separator_temperature, specific_enthalpy, 2, &self.constants);
        let temperature_k = temperature + 273.15;
        let pressure = total_vapor_moles * GAS_CONSTANT * temperature_k / COMPRESSOR_VESSEL_VOLUME;

        (temperature, pressure, vapor_composition)
    }

    /* Bloco 2 (ex-Flows, Blocks 23/24/31): vazão de recycle (slot 5), bypass (slot 6, cópia do
    5), e a curva característica + anti-surge do próprio compressor (slot 8 — fisicamente
    separador→compressor, mas é o desempenho do COMPRESSOR que decide o quanto). `enthalpy[8]`
    do Flows original (entalpia do vapor do separador, SEM a correção de Block 24) é recomputada
    aqui, fresca — mesmo padrão já usado em `derivatives.rs` pras entalpias de feed.
    */
    #[need(key = "compressor.pressure")]
    #[need(key = "reactor.pressure")]
    #[need(key = "separator.pressure")]
    #[need(key = "separator.temperature")]
    #[need(prefix = "separator.vapor_composition", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    #[need(prefix = "compressor.vapor_composition", components = ["0", "1", "2", "3", "4", "5", "6", "7"])]
    #[need(key = "valve.compressor_recycle.position")]
    #[offer(key = "flows.stream_flow.5")]
    #[offer(key = "flows.stream_flow.6")]
    #[offer(key = "flows.stream_flow.8")]
    #[offer(key = "flows.compressor_work")]
    #[offer(key = "flows.compressor_discharge_enthalpy")]
    #[allow(clippy::too_many_arguments)]
    fn outlet_flows(
        &self,
        compressor_pressure: f64,
        reactor_pressure: f64,
        separator_pressure: f64,
        separator_temperature: f64,
        separator_vapor: [f64; 8],
        compressor_vapor: [f64; 8],
        compressor_recycle_position: f64,
    ) -> (f64, f64, f64, f64, f64) {
        let mol_weight = |z: &[f64; 8]| -> f64 { (0..8).map(|i| z[i] * self.constants.xmw[i]).sum() };
        let mw5 = mol_weight(&compressor_vapor);
        let mw8 = mol_weight(&separator_vapor);

        let flow5 = 1937.6 * (compressor_pressure - reactor_pressure).max(0.0).sqrt() / mw5;
        let flow6 = flow5;

        let pressure_ratio = (compressor_pressure / separator_pressure).max(1.0).min(COMPRESSOR_PRESSURE_RATIO_MAX);
        let flow_coeff = COMPRESSOR_FLOW_MAX / 1.197;
        let mut compressor_mass_flow = COMPRESSOR_FLOW_MAX + flow_coeff * (1.0 - pressure_ratio.powi(3));
        let compressor_work = compressor_mass_flow * (separator_temperature + 273.15) * 1.8e-6 * 1.9872
            * (compressor_pressure - separator_pressure)
            / (mw8 * separator_pressure);
        compressor_mass_flow -= compressor_recycle_position * 53.349 * (compressor_pressure - separator_pressure).max(0.0).sqrt();
        compressor_mass_flow = compressor_mass_flow.max(1e-3);
        let flow8 = compressor_mass_flow / mw8;

        let separator_vapor_enthalpy = mixture_enthalpy(&separator_vapor, separator_temperature, 1, &self.constants);
        let compressor_discharge_enthalpy = separator_vapor_enthalpy + compressor_work / flow8;

        (flow5, flow6, flow8, compressor_work, compressor_discharge_enthalpy)
    }

    /* Bloco 3 (ex-Derivatives, Block 40 YP(28..36)): balanço de massa/energia do próprio estado —
    igual às outras 3 unidades, quem publica a derivada agora é a própria unidade, não mais um
    componente à parte. Precisa de tanta coisa cruzada quanto o original precisava — a física não
    muda, só quem a hospeda.
    */
    #[need(key = "flows.stream_flow.0")]
    #[need(key = "flows.stream_flow.1")]
    #[need(key = "flows.stream_flow.2")]
    #[need(key = "flows.stream_flow.4")]
    #[need(key = "flows.stream_flow.5")]
    #[need(key = "flows.stream_flow.8")]
    #[need(prefix = "flows.flash_vapor_component_flow", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    #[need(prefix = "separator.vapor_composition", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    #[need(key = "stripper.temperature")]
    #[need(key = "flows.compressor_discharge_enthalpy")]
    #[need(prefix = "compressor.vapor_composition", components = ["0", "1", "2", "3", "4", "5", "6", "7"])]
    #[need(key = "compressor.temperature")]
    #[offer(prefix = "compressor.state", components = ["0.derivative", "1.derivative", "2.derivative", "3.derivative", "4.derivative", "5.derivative", "6.derivative", "7.derivative"])]
    #[offer(key = "compressor.state.8.derivative")]
    #[allow(clippy::too_many_arguments)]
    fn yp_derivative(
        &self,
        flow0: f64,
        flow1: f64,
        flow2: f64,
        flow4: f64,
        flow5: f64,
        flow8: f64,
        flash_vapor_flow: [f64; 8],
        separator_vapor: [f64; 8],
        stripper_temperature: f64,
        compressor_discharge_enthalpy: f64,
        compressor_vapor: [f64; 8],
        compressor_temperature: f64,
    ) -> ([f64; 8], f64) {
        /* Composição do vapor do flash (slot 4) só existe normalizando FCM — mesmo cálculo de
        `derivatives.rs` pra este mesmo termo.
        */
        let flash_vapor_total: f64 = flash_vapor_flow.iter().sum();
        let mut flash_vapor_composition = [0.0f64; 8];
        if flash_vapor_total > 0.0 {
            for i in 0..8 {
                flash_vapor_composition[i] = flash_vapor_flow[i] / flash_vapor_total;
            }
        }
        let enthalpy_flash_vapor = mixture_enthalpy(&flash_vapor_composition, stripper_temperature, 1, &self.constants);
        let enthalpy_feed_d = mixture_enthalpy(&FEED_D_COMPOSITION, FEED_TEMPERATURE, 1, &self.constants);
        let enthalpy_feed_e = mixture_enthalpy(&FEED_E_COMPOSITION, FEED_TEMPERATURE, 1, &self.constants);
        let enthalpy_feed_a = mixture_enthalpy(&FEED_A_COMPOSITION, FEED_TEMPERATURE, 1, &self.constants);
        let enthalpy_compressor_recycle = mixture_enthalpy(&compressor_vapor, compressor_temperature, 1, &self.constants);

        let mut compressor_vapor_derivative = [0.0f64; 8];
        for i in 0..8 {
            compressor_vapor_derivative[i] = FEED_D_COMPOSITION[i] * flow0
                + FEED_E_COMPOSITION[i] * flow1
                + FEED_A_COMPOSITION[i] * flow2
                + flash_vapor_flow[i]
                + separator_vapor[i] * flow8
                - compressor_vapor[i] * flow5;
        }
        let compressor_enthalpy_derivative = enthalpy_feed_d * flow0
            + enthalpy_feed_e * flow1
            + enthalpy_feed_a * flow2
            + enthalpy_flash_vapor * flow4
            + compressor_discharge_enthalpy * flow8
            - enthalpy_compressor_recycle * flow5;

        (compressor_vapor_derivative, compressor_enthalpy_derivative)
    }

    /* Bloco 4 (ex-reactor_feed_analyzer.rs): XMEAS 23-28, Reactor Feed Analysis (Stream 6) — a
    composição de vapor do próprio Compressor (stream 6 = cópia bit-a-bit do bypass, Block 31 de
    teprob.f) convertida de fração molar pra mol% (teprob.f: "Units = Mole %").
    */
    #[need(prefix = "compressor.vapor_composition", components = ["0", "1", "2", "3", "4", "5"])]
    #[offer(prefix = "xmeas.stream6.component", components = ["a", "b", "c", "d", "e", "f"])]
    fn reactor_feed_analysis(&self, composition: [f64; 6]) -> [f64; 6] {
        let mut mole_percent = [0.0f64; 6];
        for i in 0..6 {
            mole_percent[i] = composition[i] * 100.0;
        }
        mole_percent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use monjolo::snapshot::Snapshot;
    use monjolo::state_registry::StateRegistry;

    /* As 3 tarefas (`thermodynamics`/`outlet_flows`/`yp_derivative`) não têm teste isolado aqui:
    quase todo `#[need]` delas é ofertado por OUTRA unidade real (Reactor/Separator/Stripper/Flows)
    descoberta pelo MESMO `inventory` — testar de verdade exigiria ou (a) reconstruir o mesmo
    conjunto de dependências via `attach_discovered_components`, que já vira um teste de PLANTA
    INTEIRA (coberto por `tests::wires_and_evaluates_without_panicking`, `src/lib.rs`, sem
    duplicar aqui), ou (b) construir os structs-tarefa privados gerados pela macro à mão (frágil —
    depende de nomes internos de campo que a macro pode mudar). Cobertura real de valor
    hand-computed pra estas 3 tarefas fica pro golden-trace de fim de migração (plano, seção
    "Verificação").
    */

    #[test]
    fn new_seeds_own_state_with_initial_condition() {
        let registry = StateRegistry::shared();
        let initial = Snapshot::from_pairs(&[
            ("state.compressor_vapor.A", 1.0),
            ("state.compressor_vapor.B", 2.0),
            ("state.compressor_vapor.C", 3.0),
            ("state.compressor_vapor.D", 4.0),
            ("state.compressor_vapor.E", 5.0),
            ("state.compressor_vapor.F", 6.0),
            ("state.compressor_vapor.G", 7.0),
            ("state.compressor_vapor.H", 8.0),
            ("state.compressor.energy", 42.0),
        ]);

        let compressor = Compressor::new(&mut registry.borrow_mut(), &initial);

        assert_eq!(compressor.vapor(), [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
        assert_eq!(compressor.enthalpy(), 42.0);
    }

    #[test]
    fn new_defaults_missing_keys_to_zero() {
        let registry = StateRegistry::shared();
        let initial = Snapshot::from_pairs(&[]);

        let compressor = Compressor::new(&mut registry.borrow_mut(), &initial);

        assert_eq!(compressor.vapor(), [0.0; 8]);
        assert_eq!(compressor.enthalpy(), 0.0);
    }
}
