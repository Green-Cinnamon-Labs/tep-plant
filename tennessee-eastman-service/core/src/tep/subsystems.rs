// dynamics/tep/subsystems.rs
//
// Subsistemas da planta TEP. Cada struct é uma unidade de processo.
// Não implementam DynamicModelV2 — recebem inputs explícitos e retornam outputs.
// O orquestrador (TepPlantCore) chama cada um em sequência e injeta outputs
// de um como inputs do próximo.

use crate::disturbance::cubic::{
    eval_disturbance, lcg_rand, update_segment, CubicDisturbanceState,
};
use crate::tep::constants::TepConstants;
use crate::tep::thermo::{liquid_density, mixture_enthalpy, temperature_from_enthalpy};
use crate::tep::ValvePositions;

// ─── Constantes físicas ───────────────────────────────────────────────────────
const REACTOR_VOLUME: f64                   = 1300.0;  // volume total do vaso do reator [m³]
const SEPARATOR_VOLUME: f64                 = 3500.0;  // volume total do separador vapor/líquido [m³]
const STRIPPER_VOLUME: f64                  = 156.5;  // volume total do stripper (usado para normalizar nível) [m³]
const COMPRESSOR_VESSEL_VOLUME: f64         = 5000.0; // volume do vaso do compressor/condensador [m³]
const GAS_CONSTANT: f64                     = 998.9;  // R em [mmHg·m³/(kmol·K)]
const COMPRESSOR_FLOW_MAX: f64              = 280275.0; // vazão mássica máxima do compressor [kg/h]
const COMPRESSOR_PRESSURE_RATIO_MAX: f64    = 1.3; // razão de pressão máxima operável
const REACTION_ENTHALPIES: [f64; 2]         = [0.06899381054, 0.05]; // calor das reações 1 e 2 [kJ/kmol]
const VALVE_FLOW_MAX: [f64; 12] = [
     400.0,  // XMV-01: feed A          — vazão máxima [kmol/h]
     400.0,  // XMV-02: feed D          — vazão máxima [kmol/h]
     100.0,  // XMV-03: feed E          — vazão máxima [kmol/h]
    1500.0,  // XMV-04: feed A/C        — vazão máxima [kmol/h]
       0.0,  // XMV-05: recycle         — usa curva característica do compressor
       0.0,  // XMV-06: sep. liquid     — usa equação hidráulica com raiz de ΔP
    1500.0,  // XMV-07: purge           — vazão máxima [kmol/h]
    1000.0,  // XMV-08: product         — vazão máxima [kmol/h]
       0.03, // XMV-09: cond. cooling   — UA máximo [kJ/(h·°C)]
    1000.0,  // XMV-10: reactor cooling — vazão máxima [kmol/h]
    1200.0,  // XMV-11: stripper steam  — vazão máxima [kmol/h]
       0.0,  // XMV-12: agitator        — não é válvula, não usa este array
];

fn stream_composition(xst: &[[f64; 13]; 8], stream: usize) -> [f64; 8] {
    let mut col = [0.0f64; 8];
    for i in 0..8 { col[i] = xst[i][stream]; }
    col
}

// ─── Output structs ───────────────────────────────────────────────────────────

/// Saídas do cálculo de perturbações (Blocks 7–12).
pub struct DisturbanceOut {
    /// Fator cinético de reação 1 — multiplica taxa de rr[0] e rr[1] (IDV 13)
    pub reaction_factor_1: f64,
    /// Fator cinético de reação 2 — multiplica taxa de rr[2] e rr[3] (IDV 13)
    pub reaction_factor_2: f64,
    /// Temperatura de entrada da água de resfriamento do reator [°C] (IDV 11)
    pub reactor_cooling_water_temp: f64,
    /// Temperatura de entrada da água de resfriamento do separador [°C] (IDV 12)
    pub separator_cooling_water_temp: f64,
}

/// Estado termodinâmico do reator (Blocks 14–18).
pub struct ReactorOut {
    /// Temperatura do líquido [°C]
    pub temperature: f64,
    /// Temperatura do líquido [K]
    pub temperature_k: f64,
    /// Pressão total do reator [mmHg]
    pub pressure: f64,
    /// Frações molares do líquido por componente [8]
    pub liquid_composition: [f64; 8],
    /// Volume do líquido [m³]
    pub liquid_volume: f64,
    /// Densidade do líquido [kmol/m³]
    pub liquid_density: f64,
    /// Volume do vapor [m³]
    pub vapor_volume: f64,
    /// Frações molares do vapor por componente [8]
    pub vapor_composition: [f64; 8],
    /// Kmol de cada componente na fase vapor [8]
    pub vapor_kmol: [f64; 8],
    /// Total de kmol na fase vapor
    pub total_vapor_kmol: f64,
    /// Taxa de reação por componente [kmol/h] — negativo = consumido [8]
    pub reaction_rates: [f64; 8],
    /// Calor gerado pelas reações [kJ/h]
    pub heat_of_reaction: f64,
}

/// Estado termodinâmico do separador vapor/líquido (Block 14).
pub struct SeparatorOut {
    /// Temperatura [°C]
    pub temperature: f64,
    /// Pressão total [mmHg]
    pub pressure: f64,
    /// Frações molares do líquido por componente [8]
    pub liquid_composition: [f64; 8],
    /// Volume do líquido [m³]
    pub liquid_volume: f64,
    /// Densidade do líquido [kmol/m³]
    pub liquid_density: f64,
    /// Frações molares do vapor por componente [8]
    pub vapor_composition: [f64; 8],
    /// Kmol de cada componente na fase vapor [8]
    pub vapor_kmol: [f64; 8],
    /// Total de kmol na fase vapor
    pub total_vapor_kmol: f64,
}

/// Estado termodinâmico do stripper (Block 15).
pub struct StripperOut {
    /// Temperatura do líquido [°C]
    pub temperature: f64,
    /// Frações molares do líquido por componente [8]
    pub liquid_composition: [f64; 8],
    /// Volume do líquido [m³]
    pub liquid_volume: f64,
    /// Densidade do líquido [kmol/m³]
    pub liquid_density: f64,
}

/// Estado termodinâmico do vaso do compressor/condensador (Block 16).
pub struct CompressorOut {
    /// Temperatura do vapor [°C]
    pub temperature: f64,
    /// Pressão total [mmHg]
    pub pressure: f64,
    /// Frações molares do vapor por componente [8]
    pub vapor_composition: [f64; 8],
}

/// Vazões e entalpias de todos os 13 streams (Blocks 22–31).
/// Índices 0–12 correspondem aos streams TEP numerados de 1 a 13.
pub struct FlowsOut {
    /// Vazão molar total de cada stream [kmol/h] — índice = stream - 1
    pub stream_flows: [f64; 13],
    /// Vazão molar por componente e stream [kmol/h] — [componente][stream]
    pub component_flows: [[f64; 13]; 8],
    /// Entalpia específica de cada stream [kJ/kmol]
    pub stream_enthalpies: [f64; 13],
    /// Peso molecular médio da mistura de cada stream [kg/kmol]
    pub stream_mol_weights: [f64; 13],
    /// Trabalho de compressão adicionado ao stream de reciclo [kJ/h]
    pub compressor_work: f64,
    /// Coeficiente global de troca do condensador UA [kJ/(h·°C)]
    pub condenser_ua: f64,
    /// Fator de agitação do reator — escala a transferência de calor da camisa
    pub agitation_factor: f64,
}

/// Transferências de calor calculadas nos Blocks 32–34.
pub struct HeatOut {
    /// Calor trocado na camisa d'água do reator [kJ/h] — positivo = resfriamento
    pub reactor_heat: f64,
    /// Calor removido no condensador do separador [kJ/h]
    pub separator_heat: f64,
    /// Calor removido no condensador do stripper [kJ/h]
    pub condenser_heat: f64,
}

// ─── TepDisturbances (Blocks 7–12) ───────────────────────────────────────────
// Atualiza o estado de perturbações e calcula as condições de alimentação.
// Modifica xst[0..2][3], tst[0], tst[3] in-place via referência mutável.

pub struct TepDisturbances;

impl TepDisturbances {
    pub fn compute(
        time: f64,
        disturbance_flags: &[i32; 20],
        step_magnitudes: &[f64; 20],
        state: &mut CubicDisturbanceState,
        stream_comps: &mut [[f64; 13]; 8],
        stream_temps: &mut [f64; 13],
    ) -> DisturbanceOut {
        let mut flags = *disturbance_flags;
        for v in flags.iter_mut() {
            *v = if *v > 0 { 1 } else { 0 };
        }

        // Block 8
        state.channels[0].active = flags[7];
        state.channels[1].active = flags[7];
        state.channels[2].active = flags[8];
        state.channels[3].active = flags[9];
        state.channels[4].active = flags[10];
        state.channels[5].active = flags[11];
        state.channels[6].active = flags[12];
        state.channels[7].active = flags[12];
        state.channels[8].active = flags[15];
        state.channels[9].active = flags[16];
        state.channels[10].active = flags[17];
        state.channels[11].active = flags[19];

        // Block 9
        for i in 0..9 {
            if time >= state.channels[i].t_next {
                let hw = state.channels[i].t_next - state.channels[i].t_last;
                let sw = state.channels[i].a
                    + hw * (state.channels[i].b + hw * (state.channels[i].c + hw * state.channels[i].d));
                let spw =
                    state.channels[i].b + hw * (2.0 * state.channels[i].c + 3.0 * hw * state.channels[i].d);
                state.channels[i].t_last = state.channels[i].t_next;
                update_segment(i, sw, spw, state);
            }
        }

        // Block 10
        for i in 9..12 {
            if time >= state.channels[i].t_next {
                let hw = state.channels[i].t_next - state.channels[i].t_last;
                let sw = state.channels[i].a
                    + hw * (state.channels[i].b + hw * (state.channels[i].c + hw * state.channels[i].d));
                let spw =
                    state.channels[i].b + hw * (2.0 * state.channels[i].c + 3.0 * hw * state.channels[i].d);
                state.channels[i].t_last = state.channels[i].t_next;
                if sw > 0.1 {
                    state.channels[i].a = sw;
                    state.channels[i].b = spw;
                    state.channels[i].c = -(3.0 * sw + 0.2 * spw) / 0.01;
                    state.channels[i].d = (2.0 * sw + 0.1 * spw) / 0.001;
                    state.channels[i].t_next = state.channels[i].t_last + 0.1;
                } else {
                    let hw2 = state.channels[i].h_span * lcg_rand(-1, state) + state.channels[i].h_zero;
                    state.channels[i].a = 0.0;
                    state.channels[i].b = 0.0;
                    state.channels[i].c = state.channels[i].active as f64 / (hw2 * hw2);
                    state.channels[i].d = 0.0;
                    state.channels[i].t_next = state.channels[i].t_last + hw2;
                }
            }
        }

        // Block 11
        if time == 0.0 {
            for i in 0..12 {
                state.channels[i].a = state.channels[i].s_zero;
                state.channels[i].b = 0.0;
                state.channels[i].c = 0.0;
                state.channels[i].d = 0.0;
                state.channels[i].t_last = 0.0;
                state.channels[i].t_next = 0.1;
            }
        }

        // Block 12
        stream_comps[0][3] =
            eval_disturbance(0, time, state) - flags[0] as f64 * 0.03 - flags[1] as f64 * 2.43719e-3;
        stream_comps[1][3] = eval_disturbance(1, time, state) + flags[1] as f64 * 0.005;
        stream_comps[2][3] = 1.0 - stream_comps[0][3] - stream_comps[1][3];
        stream_temps[0] = eval_disturbance(2, time, state) + flags[2] as f64 * step_magnitudes[2];
        stream_temps[3] = eval_disturbance(3, time, state);
        let reactor_cw_temp   = eval_disturbance(4, time, state) + flags[3] as f64 * step_magnitudes[3];
        let separator_cw_temp = eval_disturbance(5, time, state) + flags[4] as f64 * step_magnitudes[4];
        let reaction_factor_1 = eval_disturbance(6, time, state);
        let reaction_factor_2 = eval_disturbance(7, time, state);

        DisturbanceOut {
            reaction_factor_1,
            reaction_factor_2,
            reactor_cooling_water_temp:   reactor_cw_temp,
            separator_cooling_water_temp: separator_cw_temp,
        }
    }
}

// ─── TepReactor (Blocks 14–18, reatores) ─────────────────────────────────────
// state_slice[0..3]  = ucvr[0..2]  (vapor: A, B, C)
// state_slice[3..8]  = uclr[3..7]  (líquido: D, E, F, G, H)
// state_slice[8]     = etr

pub struct TepReactor;

impl TepReactor {
    pub fn compute_thermo(
        slice: &[f64],
        reaction_factor_1: f64,
        reaction_factor_2: f64,
        time: f64,
        c: &TepConstants,
    ) -> ReactorOut {
        let mut vapor_moles  = [0.0f64; 8]; // kmol A,B,C na fase vapor (estado)
        let mut liquid_moles = [0.0f64; 8]; // kmol D,E,F,G,H na fase líquida (estado)
        for i in 0..3 { vapor_moles[i]  = slice[i]; }
        for i in 3..8 { liquid_moles[i] = slice[i]; }
        let total_enthalpy = slice[8];

        let total_liquid_moles: f64 = liquid_moles.iter().sum();
        let mut liquid_composition = [0.0f64; 8];
        for i in 0..8 { liquid_composition[i] = liquid_moles[i] / total_liquid_moles; }

        let specific_enthalpy = total_enthalpy / total_liquid_moles;
        let temperature_init = if time == 0.0 { 120.0 } else { 0.0 };
        let temperature   = temperature_from_enthalpy(&liquid_composition, temperature_init, specific_enthalpy, 0, c);
        let temperature_k = temperature + 273.15;
        let density       = liquid_density(&liquid_composition, temperature, c);
        let volume_liquid = total_liquid_moles / density;
        let volume_vapor  = REACTOR_VOLUME - volume_liquid;

        let mut partial_pressures = [0.0f64; 8];
        let mut pressure = 0.0f64;
        for i in 0..3 {
            partial_pressures[i] = vapor_moles[i] * GAS_CONSTANT* temperature_k / volume_vapor;
            pressure += partial_pressures[i];
        }
        for i in 3..8 {
            partial_pressures[i] = (c.avp[i] + c.bvp[i] / (temperature + c.cvp[i])).exp() * liquid_composition[i];
            pressure += partial_pressures[i];
        }

        let mut vapor_composition = [0.0f64; 8];
        for i in 0..8 { vapor_composition[i] = partial_pressures[i] / pressure; }
        let total_vapor_moles = pressure * volume_vapor / GAS_CONSTANT/ temperature_k;
        for i in 3..8 { vapor_moles[i] = total_vapor_moles * vapor_composition[i]; }

        // Cinética de Arrhenius — taxas brutas das 4 reações
        let mut rates = [0.0f64; 4];
        rates[0] = (31.5859536 - 40000.0 / 1.987 / temperature_k).exp() * reaction_factor_1;
        rates[1] = (3.00094014 - 20000.0 / 1.987 / temperature_k).exp() * reaction_factor_2;
        rates[2] = (53.4060443 - 60000.0 / 1.987 / temperature_k).exp();
        rates[3] = rates[2] * 0.767488334;
        if partial_pressures[0] > 0.0 && partial_pressures[2] > 0.0 {
            let rf1 = partial_pressures[0].powf(1.1544);
            let rf2 = partial_pressures[2].powf(0.3735);
            rates[0] *= rf1 * rf2 * partial_pressures[3];
            rates[1] *= rf1 * rf2 * partial_pressures[4];
        } else {
            rates[0] = 0.0;
            rates[1] = 0.0;
        }
        rates[2] *= partial_pressures[0] * partial_pressures[4];
        rates[3] *= partial_pressures[0] * partial_pressures[3];
        for r in rates.iter_mut() { *r *= volume_vapor; }

        // Estequiometria: consumo/produção por componente
        let mut reaction_rates = [0.0f64; 8];
        reaction_rates[0] = -rates[0] - rates[1] - rates[2];
        reaction_rates[2] = -rates[0] - rates[1];
        reaction_rates[3] = -rates[0] - 1.5 * rates[3];
        reaction_rates[4] = -rates[1] - rates[2];
        reaction_rates[5] =  rates[2] + rates[3];
        reaction_rates[6] =  rates[0];
        reaction_rates[7] =  rates[1];
        let heat_of_reaction = rates[0] * REACTION_ENTHALPIES[0] + rates[1] * REACTION_ENTHALPIES[1];

        ReactorOut {
            temperature,
            temperature_k,
            pressure,
            liquid_composition,
            liquid_volume: volume_liquid,
            liquid_density: density,
            vapor_volume: volume_vapor,
            vapor_composition,
            vapor_kmol: vapor_moles,
            total_vapor_kmol: total_vapor_moles,
            reaction_rates,
            heat_of_reaction,
        }
    }

    pub fn compute_derivatives(flows: &FlowsOut, reactor: &ReactorOut, reactor_heat: f64) -> Vec<f64> {
        let mut yp = vec![0.0f64; 9];
        for i in 0..8 {
            yp[i] = flows.component_flows[i][6] - flows.component_flows[i][7] + reactor.reaction_rates[i];
        }
        yp[8] = flows.stream_enthalpies[6] * flows.stream_flows[6] - flows.stream_enthalpies[7] * flows.stream_flows[7] + reactor.heat_of_reaction+ reactor_heat;
        yp
    }
}

// ─── TepSeparator ─────────────────────────────────────────────────────────────
// state_slice[0..3]  = ucvs[0..2], state_slice[3..8] = ucls[3..7], state_slice[8] = ets

pub struct TepSeparator;

impl TepSeparator {
    pub fn compute_thermo(slice: &[f64], reactor_temperature: f64, c: &TepConstants) -> SeparatorOut {
        let mut vapor_moles = [0.0f64; 8];   // kmol de A,B,C na fase vapor (estado)
        let mut liquid_moles = [0.0f64; 8];  // kmol de D,E,F,G,H na fase líquida (estado)
        for i in 0..3 {
            vapor_moles[i] = slice[i];
        }
        for i in 3..8 {
            liquid_moles[i] = slice[i];
        }
        let total_enthalpy = slice[8];

        let total_liquid_moles: f64 = liquid_moles.iter().sum();
        let mut liquid_composition = [0.0f64; 8];
        for i in 0..8 {
            liquid_composition[i] = liquid_moles[i] / total_liquid_moles;
        }

        let specific_enthalpy = total_enthalpy / total_liquid_moles;
        let temperature = temperature_from_enthalpy(&liquid_composition, reactor_temperature, specific_enthalpy, 0, c);
        let temperature_k = temperature + 273.15;
        let density = liquid_density(&liquid_composition, temperature, c);
        let volume_liquid = total_liquid_moles / density;
        let volume_vapor = SEPARATOR_VOLUME - volume_liquid;

        let mut partial_pressures = [0.0f64; 8];
        let mut pressure = 0.0f64;
        for i in 0..3 {
            partial_pressures[i] = vapor_moles[i] * GAS_CONSTANT* temperature_k / volume_vapor;
            pressure += partial_pressures[i];
        }
        for i in 3..8 {
            partial_pressures[i] = (c.avp[i] + c.bvp[i] / (temperature + c.cvp[i])).exp() * liquid_composition[i];
            pressure += partial_pressures[i];
        }

        let mut vapor_composition = [0.0f64; 8];
        for i in 0..8 {
            vapor_composition[i] = partial_pressures[i] / pressure;
        }
        let total_vapor_moles = pressure * volume_vapor / GAS_CONSTANT/ temperature_k;
        for i in 3..8 {
            vapor_moles[i] = total_vapor_moles * vapor_composition[i];
        }

        SeparatorOut {
            temperature,
            pressure,
            liquid_composition,
            liquid_volume: volume_liquid,
            liquid_density: density,
            vapor_composition,
            vapor_kmol: vapor_moles,
            total_vapor_kmol: total_vapor_moles,
        }
    }

    pub fn compute_derivatives(flows: &FlowsOut, separator_heat: f64) -> Vec<f64> {
        let mut yp = vec![0.0f64; 9];
        for i in 0..8 {
            yp[i] = flows.component_flows[i][7] - flows.component_flows[i][8] - flows.component_flows[i][9] - flows.component_flows[i][10];
        }
        yp[8] =
            flows.stream_enthalpies[7] * flows.stream_flows[7] - flows.stream_enthalpies[8] * flows.stream_flows[8] - flows.stream_enthalpies[9] * flows.stream_flows[9] - flows.stream_enthalpies[10] * flows.stream_flows[10]
                + separator_heat;
        yp
    }
}

// ─── TepStripper ──────────────────────────────────────────────────────────────
// state_slice[0..8] = uclc[0..7], state_slice[8] = etc

pub struct TepStripper;

impl TepStripper {
    pub fn compute_thermo(slice: &[f64], separator_temperature: f64, c: &TepConstants) -> StripperOut {
        let mut liquid_moles = [0.0f64; 8];
        for i in 0..8 { liquid_moles[i] = slice[i]; }
        let total_enthalpy = slice[8];

        let total_liquid_moles: f64 = liquid_moles.iter().sum();
        let mut liquid_composition = [0.0f64; 8];
        for i in 0..8 { liquid_composition[i] = liquid_moles[i] / total_liquid_moles; }

        let specific_enthalpy = total_enthalpy / total_liquid_moles;
        let temperature = temperature_from_enthalpy(&liquid_composition, separator_temperature, specific_enthalpy, 0, c);
        let density     = liquid_density(&liquid_composition, temperature, c);
        let volume_liquid = total_liquid_moles / density;

        StripperOut { temperature, liquid_composition, liquid_volume: volume_liquid, liquid_density: density }
    }

    pub fn compute_derivatives(flows: &FlowsOut, condenser_heat: f64) -> Vec<f64> {
        let mut yp = vec![0.0f64; 9];
        for i in 0..8 {
            yp[i] = flows.component_flows[i][11] - flows.component_flows[i][12];
        }
        yp[8] = flows.stream_enthalpies[3] * flows.stream_flows[3] + flows.stream_enthalpies[10] * flows.stream_flows[10]
            - flows.stream_enthalpies[4] * flows.stream_flows[4]
            - flows.stream_enthalpies[12] * flows.stream_flows[12]
            + condenser_heat;
        yp
    }
}

// ─── TepCompressor ────────────────────────────────────────────────────────────
// state_slice[0..8] = ucvv[0..7], state_slice[8] = etv

pub struct TepCompressor;

impl TepCompressor {
    pub fn compute_thermo(slice: &[f64], separator_temperature: f64, c: &TepConstants) -> CompressorOut {
        let mut vapor_moles = [0.0f64; 8];
        for i in 0..8 { vapor_moles[i] = slice[i]; }
        let total_enthalpy = slice[8];

        let total_vapor_moles: f64 = vapor_moles.iter().sum();
        let mut vapor_composition = [0.0f64; 8];
        for i in 0..8 { vapor_composition[i] = vapor_moles[i] / total_vapor_moles; }

        let specific_enthalpy = total_enthalpy / total_vapor_moles;
        let temperature   = temperature_from_enthalpy(&vapor_composition, separator_temperature, specific_enthalpy, 2, c);
        let temperature_k = temperature + 273.15;
        let pressure      = total_vapor_moles * GAS_CONSTANT* temperature_k / COMPRESSOR_VESSEL_VOLUME;

        CompressorOut { temperature, pressure, vapor_composition }
    }

    pub fn compute_derivatives(flows: &FlowsOut) -> Vec<f64> {
        let mut yp = vec![0.0f64; 9];
        for i in 0..8 {
            yp[i] =
                flows.component_flows[i][0] + flows.component_flows[i][1] + flows.component_flows[i][2] + flows.component_flows[i][4] + flows.component_flows[i][8] - flows.component_flows[i][5];
        }
        yp[8] = flows.stream_enthalpies[0] * flows.stream_flows[0]
            + flows.stream_enthalpies[1] * flows.stream_flows[1]
            + flows.stream_enthalpies[2] * flows.stream_flows[2]
            + flows.stream_enthalpies[4] * flows.stream_flows[4]
            + flows.stream_enthalpies[8] * flows.stream_flows[8]
            - flows.stream_enthalpies[5] * flows.stream_flows[5];
        yp
    }
}

// ─── TepFlows (Blocks 19–31) ─────────────────────────────────────────────────
// Calcula composições de correntes, fluxos mássicos e fracionamento no stripper.
// Modifica xst e tst in-place (colunas 4–12) e sfr.

pub struct TepFlows;

impl TepFlows {
    #[allow(clippy::too_many_arguments)]
    pub fn compute(
        reactor: &ReactorOut,
        separator: &SeparatorOut,
        stripper: &StripperOut,
        compressor: &CompressorOut,
        xst: &mut [[f64; 13]; 8],
        tst: &mut [f64; 13],
        sfr: &mut [f64; 8],
        vpos: &ValvePositions,
        idv: &[i32; 20],
        c: &TepConstants,
        time: f64,
        ds: &CubicDisturbanceState,
    ) -> FlowsOut {

        // Block 19: composições de corrente a partir dos estados de unidade
        for i in 0..8 {
            xst[i][5]  = compressor.vapor_composition[i];
            xst[i][7]  = reactor.vapor_composition[i];
            xst[i][8]  = separator.vapor_composition[i];
            xst[i][9]  = separator.vapor_composition[i];
            xst[i][10] = separator.liquid_composition[i];
            xst[i][12] = stripper.liquid_composition[i];
        }

        // Block 20: pesos moleculares médios de cada stream
        let mut mol_weights = [0.0f64; 13];
        for i in 0..8 {
            mol_weights[0] += xst[i][0] * c.xmw[i];
            mol_weights[1] += xst[i][1] * c.xmw[i];
            mol_weights[5] += xst[i][5] * c.xmw[i];
            mol_weights[7] += xst[i][7] * c.xmw[i];
            mol_weights[8] += xst[i][8] * c.xmw[i];
            mol_weights[9] += xst[i][9] * c.xmw[i];
        }

        // Block 21: temperaturas e entalpias das correntes
        tst[5]  = compressor.temperature;  // stream 6: feed do reator (saída do compressor)
        tst[7]  = reactor.temperature;     // stream 8: efluente do reator
        tst[8]  = separator.temperature;   // stream 9: vapor de topo do separador
        tst[9]  = separator.temperature;   // stream 10: líquido do separador (mesma temp. do topo)
        tst[10] = separator.temperature;   // stream 11: feed do stripper (mesmo líquido)
        tst[12] = stripper.temperature;    // stream 13: produto de fundo do stripper
        let mut enthalpies = [0.0f64; 13];
        enthalpies[0]  = mixture_enthalpy(&stream_composition(xst, 0),  tst[0],  1, c); // stream 1: feed A (vapor)
        enthalpies[1]  = mixture_enthalpy(&stream_composition(xst, 1),  tst[1],  1, c); // stream 2: feed D (vapor)
        enthalpies[2]  = mixture_enthalpy(&stream_composition(xst, 2),  tst[2],  1, c); // stream 3: feed E (vapor)
        enthalpies[3]  = mixture_enthalpy(&stream_composition(xst, 3),  tst[3],  1, c); // stream 4: feed A/C (vapor)
        enthalpies[5]  = mixture_enthalpy(&stream_composition(xst, 5),  tst[5],  1, c); // stream 6: feed do reator (vapor)
        enthalpies[7]  = mixture_enthalpy(&stream_composition(xst, 7),  tst[7],  1, c); // stream 8: efluente do reator (vapor)
        enthalpies[8]  = mixture_enthalpy(&stream_composition(xst, 8),  tst[8],  1, c); // stream 9: topo do separador (vapor)
        enthalpies[9]  = enthalpies[8];                                        // stream 10: igual ao topo do separador
        enthalpies[10] = mixture_enthalpy(&stream_composition(xst, 10), tst[10], 0, c); // stream 11: líquido para stripper
        enthalpies[12] = mixture_enthalpy(&stream_composition(xst, 12), tst[12], 0, c); // stream 13: produto do stripper (líquido)

        // Blocks 22–24: vazões molares [kmol/h]
        let mut flows = [0.0f64; 13];
        flows[0]  = vpos.a_feed * VALVE_FLOW_MAX[0] / 100.0;   // feed A (gás)
        flows[1]  = vpos.d_feed * VALVE_FLOW_MAX[1] / 100.0;   // feed D (líquido)
        flows[2]  = vpos.e_feed * (1.0 - idv[5] as f64) * VALVE_FLOW_MAX[2] / 100.0;                // feed E; IDV6 bloqueia
        flows[3]  = vpos.c_feed * (1.0 - idv[6] as f64 * 0.2) * VALVE_FLOW_MAX[3] / 100.0 + 1e-10; // feed A/C; IDV7 reduz 20%
        flows[10] = vpos.purge   * VALVE_FLOW_MAX[6] / 100.0;  // purga (separador → saída)
        flows[12] = vpos.product * VALVE_FLOW_MAX[7] / 100.0;  // produto (stripper → saída)
        let condenser_ua    = vpos.condenser_cooling * VALVE_FLOW_MAX[8] * (1.0 + eval_disturbance(8, time, ds)) / 100.0;
        let agitation_factor = (vpos.agitator + 150.0) / 100.0;

        flows[5] = 1937.6  * (compressor.pressure - reactor.pressure).max(0.0).sqrt() / mol_weights[5]; // feed do reator — ΔP compressor→reator
        flows[7] = 4574.21 * (reactor.pressure   - separator.pressure).max(0.0).sqrt() * (1.0 - 0.25 * eval_disturbance(11, time, ds)) / mol_weights[7]; // reator→separador — ΔP + IDV20
        flows[9] = vpos.separator_liquid * 0.151169 * (separator.pressure - 760.0).max(0.0).sqrt() / mol_weights[9]; // líquido separador→stripper

        // Compressor (curva característica + anti-surge)
        let pressure_ratio = (compressor.pressure / separator.pressure).max(1.0).min(COMPRESSOR_PRESSURE_RATIO_MAX);
        let flow_coeff     = COMPRESSOR_FLOW_MAX / 1.197;
        let mut compressor_mass_flow = COMPRESSOR_FLOW_MAX + flow_coeff * (1.0 - pressure_ratio.powi(3));
        let compressor_work = compressor_mass_flow * (separator.temperature + 273.15) * 1.8e-6 * 1.9872
            * (compressor.pressure - separator.pressure) / (mol_weights[8] * separator.pressure);
        compressor_mass_flow -= vpos.recycle * 53.349 * (compressor.pressure - separator.pressure).max(0.0).sqrt(); // anti-surge
        compressor_mass_flow  = compressor_mass_flow.max(1e-3);
        flows[8]       = compressor_mass_flow / mol_weights[8];   // kg/h → kmol/h
        enthalpies[8] += compressor_work / flows[8];              // adiciona calor de compressão

        // Block 25: fluxos por componente
        let mut comp_flows = [[0.0f64; 13]; 8];
        for i in 0..8 {
            comp_flows[i][0]  = xst[i][0]  * flows[0];
            comp_flows[i][1]  = xst[i][1]  * flows[1];
            comp_flows[i][2]  = xst[i][2]  * flows[2];
            comp_flows[i][3]  = xst[i][3]  * flows[3];
            comp_flows[i][5]  = xst[i][5]  * flows[5];
            comp_flows[i][7]  = xst[i][7]  * flows[7];
            comp_flows[i][8]  = xst[i][8]  * flows[8];
            comp_flows[i][9]  = xst[i][9]  * flows[9];
            comp_flows[i][10] = xst[i][10] * flows[10];
            comp_flows[i][12] = xst[i][12] * flows[12];
        }

        // Blocks 26–31: fracionamento vapor/líquido no stripper
        if flows[10] > 0.1 {
            let temperature_factor = if stripper.temperature > 170.0 {
                stripper.temperature - 120.262
            } else if stripper.temperature < 5.292 {
                0.1
            } else {
                363.744 / (177.0 - stripper.temperature) - 2.22579488
            };
            let vapor_over_liquid = flows[3] / flows[10] * temperature_factor;
            sfr[3] =  8.5010 * vapor_over_liquid / (1.0 +  8.5010 * vapor_over_liquid);
            sfr[4] = 11.402  * vapor_over_liquid / (1.0 + 11.402  * vapor_over_liquid);
            sfr[5] = 11.795  * vapor_over_liquid / (1.0 + 11.795  * vapor_over_liquid);
            sfr[6] =  0.0480 * vapor_over_liquid / (1.0 +  0.0480 * vapor_over_liquid);
            sfr[7] =  0.0242 * vapor_over_liquid / (1.0 +  0.0242 * vapor_over_liquid);
        } else {
            sfr[3] = 0.9999;
            sfr[4] = 0.999;
            sfr[5] = 0.999;
            sfr[6] = 0.99;
            sfr[7] = 0.98;
        }

        let mut stripper_inlet = [0.0f64; 8]; // kmol/h de cada componente entrando no stripper
        for i in 0..8 { stripper_inlet[i] = comp_flows[i][3] + comp_flows[i][10]; }
        flows[4]  = 0.0;
        flows[11] = 0.0;
        for i in 0..8 {
            comp_flows[i][4]  = sfr[i] * stripper_inlet[i];
            comp_flows[i][11] = stripper_inlet[i] - comp_flows[i][4];
            flows[4]  += comp_flows[i][4];
            flows[11] += comp_flows[i][11];
        }
        for i in 0..8 {
            xst[i][4]  = comp_flows[i][4]  / flows[4];
            xst[i][11] = comp_flows[i][11] / flows[11];
        }
        tst[4] = stripper.temperature;
        tst[11] = stripper.temperature;
        enthalpies[4]  = mixture_enthalpy(&stream_composition(xst, 4),  tst[4],  1, c);
        enthalpies[11] = mixture_enthalpy(&stream_composition(xst, 11), tst[11], 0, c);
        flows[6]  = flows[5];
        enthalpies[6]  = enthalpies[5];
        tst[6] = tst[5];
        for i in 0..8 {
            xst[i][6]       = xst[i][5];
            comp_flows[i][6] = comp_flows[i][5];
        }

        FlowsOut {
            stream_flows: flows,
            component_flows: comp_flows,
            stream_enthalpies: enthalpies,
            stream_mol_weights: mol_weights,
            compressor_work,
            condenser_ua,
            agitation_factor,
        }
    }
}

// ─── TepHeat (Blocks 32–34) ───────────────────────────────────────────────────
// Calcula transferência de calor no reator, separador e stripper.

pub struct TepHeat;

impl TepHeat {
    pub fn compute(
        reactor: &ReactorOut,
        stripper: &StripperOut,
        flows: &FlowsOut,
        reactor_cw_return: f64,    // state[36] — temperatura de retorno da água de resfriamento do reator
        separator_cw_return: f64,  // state[37] — temperatura de retorno do condensador/separador
        reactor_temperature: f64,  // = reactor.temperature (repetido para não re-extrair do struct)
        time: f64,
        ds: &CubicDisturbanceState,
    ) -> HeatOut {
        // Block 32: reator
        let uarlev = if reactor.liquid_volume / 7.8 > 50.0 {
            1.0
        } else if reactor.liquid_volume / 7.8 < 10.0 {
            0.0
        } else {
            0.025 * reactor.liquid_volume / 7.8 - 0.25
        };
        let uar = uarlev * (-0.5 * flows.agitation_factor * flows.agitation_factor + 2.75 * flows.agitation_factor - 2.5) * 855490e-6;
        let reactor_heat = uar * (reactor_cw_return - reactor.temperature) * (1.0 - 0.35 * eval_disturbance(9, time, ds));

        // Block 33: separador
        let uas = 0.404655 * (1.0 - 1.0 / (1.0 + (flows.stream_flows[7] / 3528.73).powi(4)));
        let separator_heat = uas * (separator_cw_return - reactor_temperature) * (1.0 - 0.25 * eval_disturbance(10, time, ds));

        // Block 34: stripper (condensador)
        let condenser_heat = if stripper.temperature < 100.0 {
            flows.condenser_ua * (100.0 - stripper.temperature)
        } else {
            0.0
        };

        HeatOut { reactor_heat, separator_heat, condenser_heat }
    }
}

// ─── TepMeasurements (Block 35) ──────────────────────────────────────────────
// Snapshot físico — sem ruído. Preenche xmeas[0..22].

pub struct TepMeasurements;

impl TepMeasurements {
    pub fn compute(
        reactor: &ReactorOut,
        separator: &SeparatorOut,
        stripper: &StripperOut,
        compressor: &CompressorOut,
        flows: &FlowsOut,
        heat: &HeatOut,
        reactor_cw_return: f64,
        separator_cw_return: f64,
        xmeas: &mut [f64; 41],
    ) -> bool {
        xmeas[0] = flows.stream_flows[2] * 0.359 / 35.3145;
        xmeas[1] = flows.stream_flows[0] * flows.stream_mol_weights[0] * 0.454;
        xmeas[2] = flows.stream_flows[1] * flows.stream_mol_weights[1] * 0.454;
        xmeas[3] = flows.stream_flows[3] * 0.359 / 35.3145;
        xmeas[4] = flows.stream_flows[8] * 0.359 / 35.3145;
        xmeas[5] = flows.stream_flows[5] * 0.359 / 35.3145;
        xmeas[6] = (reactor.pressure - 760.0) / 760.0 * 101.325;
        xmeas[7] = (reactor.liquid_volume - 84.6) / 666.7 * 100.0;
        xmeas[8] = reactor.temperature;
        xmeas[9] = flows.stream_flows[9] * 0.359 / 35.3145;
        xmeas[10] = separator.temperature;
        xmeas[11] = (separator.liquid_volume - 27.5) / 290.0 * 100.0;
        xmeas[12] = (separator.pressure - 760.0) / 760.0 * 101.325;
        xmeas[13] = flows.stream_flows[10] / separator.liquid_density / 35.3145;
        xmeas[14] = (stripper.liquid_volume - 78.25) / STRIPPER_VOLUME * 100.0;
        xmeas[15] = (compressor.pressure - 760.0) / 760.0 * 101.325;
        xmeas[16] = flows.stream_flows[12] / stripper.liquid_density / 35.3145;
        xmeas[17] = stripper.temperature;
        xmeas[18] = heat.condenser_heat * 1.04e3 * 0.454;
        xmeas[19] = flows.compressor_work * 0.29307e3;
        xmeas[20] = reactor_cw_return;
        xmeas[21] = separator_cw_return;

        // Block 36: detecção de shutdown
        xmeas[6] > 3000.0
            || reactor.liquid_volume / 35.3145 > 24.0
            || reactor.liquid_volume / 35.3145 < 2.0
            || xmeas[8] > 175.0
            || separator.liquid_volume / 35.3145 > 12.0
            || separator.liquid_volume / 35.3145 < 1.0
            || stripper.liquid_volume / 35.3145 > 8.0
            || stripper.liquid_volume / 35.3145 < 1.0
    }
}
