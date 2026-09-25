/* tep/units/compressor.rs */

use crate::physics::constants::{TepConstants, TEP_SPECIES};
use crate::units::feed::{FEED_A_COMPOSITION, FEED_D_COMPOSITION, FEED_E_COMPOSITION, FEED_TEMPERATURE};
use monjolo::chemistry::{temperature_from_enthalpy, Mixture, Phase};

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
    /* Bloco 1: balanço de energia próprio → temperatura/pressão/composição — igual ao `compute()`
    monolítico de antes, só que agora é UMA tarefa entre várias, com seu próprio `needs`/`offers`.
    */
    #[task]
    fn physical_state(&self) {
        let vapor = Mixture::new(self.vapor(), Phase::Vapor, &TEP_SPECIES);
        let vapor_composition = vapor.mole_fractions();
        let total_vapor_moles = vapor.total();

        let specific_enthalpy = self.enthalpy() / total_vapor_moles;
        let temperature = temperature_from_enthalpy(&vapor_composition.as_array(), need::separator__temperature, specific_enthalpy, 2, &self.constants);
        let temperature_k = temperature + 273.15;
        let pressure = total_vapor_moles * GAS_CONSTANT * temperature_k / COMPRESSOR_VESSEL_VOLUME;

        offer::compressor__temperature = temperature;
        offer::compressor__pressure = pressure;
        offer::compressor__vapor_composition::<Vapor> = vapor_composition;
    }

    /* Bloco 2 (ex-Flows, Blocks 23/24/31): vazão de recycle (slot 5), bypass (slot 6, cópia do
    5), e a curva característica + anti-surge do próprio compressor (slot 8 — fisicamente
    separador→compressor, mas é o desempenho do COMPRESSOR que decide o quanto). `enthalpy[8]`
    do Flows original (entalpia do vapor do separador, SEM a correção de Block 24) é recomputada
    aqui, fresca — mesmo padrão já usado em `derivatives.rs` pras entalpias de feed.
    */
    #[task]
    fn outlet_flows(&self) {
        let compressor_pressure = need::compressor__pressure;
        let separator_pressure = need::separator__pressure;
        let separator_temperature = need::separator__temperature;
        let separator_vapor = need::separator__vapor_composition::<Vapor>;
        let compressor_vapor = need::compressor__vapor_composition::<Vapor>;

        let mw5 = compressor_vapor.dot(&self.constants.xmw);
        let mw8 = separator_vapor.dot(&self.constants.xmw);

        let flow5 = 1937.6 * (compressor_pressure - need::reactor__pressure).max(0.0).sqrt() / mw5;
        let flow6 = flow5;

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

        offer::flows__stream_flow__5 = flow5;
        offer::flows__stream_flow__6 = flow6;
        offer::flows__stream_flow__8 = flow8;
        offer::flows__compressor_work = compressor_work;
        offer::flows__compressor_discharge_enthalpy = compressor_discharge_enthalpy;
    }

    /* Bloco 3 (ex-Derivatives, Block 40 YP(28..36)): balanço de massa/energia do próprio estado —
    igual às outras 3 unidades, quem publica a derivada agora é a própria unidade, não mais um
    componente à parte. Precisa de tanta coisa cruzada quanto o original precisava — a física não
    muda, só quem a hospeda.
    */
    #[task]
    fn mass_and_energy_balance(&self) {
        let flow0 = need::flows__stream_flow__0;
        let flow1 = need::flows__stream_flow__1;
        let flow2 = need::flows__stream_flow__2;
        let flow4 = need::flows__stream_flow__4;
        let flow5 = need::flows__stream_flow__5;
        let flow8 = need::flows__stream_flow__8;
        let flash_vapor_flow = need::flows__flash_vapor_component_flow::<Vapor>;
        let separator_vapor = need::separator__vapor_composition::<Vapor>;
        let compressor_vapor = need::compressor__vapor_composition::<Vapor>;

        /* Composição do vapor do flash (slot 4) só existe normalizando FCM — mesmo cálculo de
        `derivatives.rs` pra este mesmo termo.
        */
        let flash_vapor_total = flash_vapor_flow.total();
        let mut flash_vapor_composition = [0.0f64; 8];
        if flash_vapor_total > 0.0 {
            for i in 0..8 {
                flash_vapor_composition[i] = flash_vapor_flow.component(i) / flash_vapor_total;
            }
        }
        let enthalpy_flash_vapor = Mixture::new(flash_vapor_composition, Phase::Vapor, &TEP_SPECIES).enthalpy(need::stripper__temperature, 1, &self.constants);
        let enthalpy_feed_d = Mixture::new(FEED_D_COMPOSITION, Phase::Vapor, &TEP_SPECIES).enthalpy(FEED_TEMPERATURE, 1, &self.constants);
        let enthalpy_feed_e = Mixture::new(FEED_E_COMPOSITION, Phase::Vapor, &TEP_SPECIES).enthalpy(FEED_TEMPERATURE, 1, &self.constants);
        let enthalpy_feed_a = Mixture::new(FEED_A_COMPOSITION, Phase::Vapor, &TEP_SPECIES).enthalpy(FEED_TEMPERATURE, 1, &self.constants);
        let enthalpy_compressor_recycle = compressor_vapor.enthalpy(need::compressor__temperature, 1, &self.constants);

        let mut derivative = [0.0f64; 8];
        for i in 0..8 {
            derivative[i] = FEED_D_COMPOSITION[i] * flow0
                + FEED_E_COMPOSITION[i] * flow1
                + FEED_A_COMPOSITION[i] * flow2
                + flash_vapor_flow.component(i)
                + separator_vapor.component(i) * flow8
                - compressor_vapor.component(i) * flow5;
        }

        offer::compressor__state__0__derivative = derivative[0];
        offer::compressor__state__1__derivative = derivative[1];
        offer::compressor__state__2__derivative = derivative[2];
        offer::compressor__state__3__derivative = derivative[3];
        offer::compressor__state__4__derivative = derivative[4];
        offer::compressor__state__5__derivative = derivative[5];
        offer::compressor__state__6__derivative = derivative[6];
        offer::compressor__state__7__derivative = derivative[7];
        offer::compressor__state__8__derivative = enthalpy_feed_d * flow0
            + enthalpy_feed_e * flow1
            + enthalpy_feed_a * flow2
            + enthalpy_flash_vapor * flow4
            + need::flows__compressor_discharge_enthalpy * flow8
            - enthalpy_compressor_recycle * flow5;
    }

    /* Bloco 4 (ex-reactor_feed_analyzer.rs): XMEAS 23-28, Reactor Feed Analysis (Stream 6) — a
    composição de vapor do próprio Compressor (stream 6 = cópia bit-a-bit do bypass, Block 31 de
    teprob.f) convertida de fração molar pra mol% (teprob.f: "Units = Mole %"). Só os 6 primeiros
    componentes (A-F) saem como XMEAS.
    */
    #[task]
    fn reactor_feed_analysis(&self) {
        let composition = need::compressor__vapor_composition::<Vapor>;
        offer::xmeas__stream6__component__a = composition.component(0) * 100.0;
        offer::xmeas__stream6__component__b = composition.component(1) * 100.0;
        offer::xmeas__stream6__component__c = composition.component(2) * 100.0;
        offer::xmeas__stream6__component__d = composition.component(3) * 100.0;
        offer::xmeas__stream6__component__e = composition.component(4) * 100.0;
        offer::xmeas__stream6__component__f = composition.component(5) * 100.0;
    }

    /* Bloco 5 (ex-measured.rs, Block 35): XMEAS 5 (Recycle Flow, stream8), 6 (Reactor Feed Rate,
    stream6), 20 (Compressor Work) — todos vazões/potência do próprio Compressor. XMEAS 16
    (Stripper Pressure) é a exceção conhecida do original: o nome sugere Stripper, mas o valor É a
    pressão do Compressor (teprob.f, não é bug de digitação — preservado tal qual, mesma chave
    externa `xmeas.stripper.pressure` de sempre).
    */
    #[task]
    fn xmeas_readings(&self) {
        offer::xmeas__stream8__flow_rate = need::flows__stream_flow__8 * 0.359 / 35.3145;
        offer::xmeas__stream6__flow_rate = need::flows__stream_flow__5 * 0.359 / 35.3145;
        offer::xmeas__stripper__pressure = (need::compressor__pressure - 760.0) / 760.0 * 101.325;
        offer::xmeas__compressor__work = need::flows__compressor_work * 0.29307e3;
    }
}
