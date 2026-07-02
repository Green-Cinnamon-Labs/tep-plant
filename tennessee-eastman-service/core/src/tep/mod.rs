// core/tep/mod.rs
//
// TennesseeEastmanModel — implementação concreta de DynamicModel para a planta TEP.
//
// State layout (50 elements):
//   [0..9]   — reator  (ucvr[0..3], uclr[3..8], etr)
//   [9..18]  — separador (ucvs[0..3], ucls[3..8], ets)
//   [18..27] — stripper (uclc[0..8], etc)
//   [27..36] — compressor/condensador (ucvv[0..8], etv)
//   [36]     — reactor_cw_return   (temperatura de retorno da água de resfriamento do reator)
//   [37]     — separator_cw_return (temperatura de retorno da água de resfriamento do condensador)
//   [38..49] — posições das válvulas XMV[0..11]  (Valve, 1 estado cada)
//   [49]     — velocidade do agitador XMV[12]     (Agitator, 1 estado)

pub mod constants;
pub mod disturbance_state;
pub mod initial_state;
pub mod subsystems;
pub mod thermo;

use crate::actuator::dynamic::{Agitator, Valve};
use crate::model::DynamicModel;
use crate::tep::constants::TepConstants;
use crate::tep::disturbance_state::TepDisturbanceState;
use crate::tep::initial_state::InitialState;
use crate::tep::subsystems::{
    TepCompressor, TepDisturbances, TepFlows, TepHeat, TepMeasurements, TepReactor, TepSeparator,
    TepStripper,
};

const VTAU: [f64; 11] = [
    8.0 / 3600.0,
    8.0 / 3600.0,
    6.0 / 3600.0,
    9.0 / 3600.0,
    7.0 / 3600.0,
    5.0 / 3600.0,
    5.0 / 3600.0,
    5.0 / 3600.0,
    120.0 / 3600.0,
    5.0 / 3600.0,
    5.0 / 3600.0,
];

const AGITATOR_TAU: f64 = 5.0 / 3600.0;

fn initial_xst() -> [[f64; 13]; 8] {
    let mut xst = [[0.0f64; 13]; 8];
    xst[1][0] = 0.0001;
    xst[3][0] = 0.9999;
    xst[4][1] = 0.9999;
    xst[5][1] = 0.0001;
    xst[0][2] = 0.9999;
    xst[1][2] = 0.0001;
    xst[0][3] = 0.485;
    xst[1][3] = 0.005;
    xst[2][3] = 0.510;
    xst
}

fn initial_tst() -> [f64; 13] {
    let mut tst = [0.0f64; 13];
    tst[0] = 45.0;
    tst[1] = 45.0;
    tst[2] = 45.0;
    tst[3] = 45.0;
    tst
}

fn initial_sfr() -> [f64; 8] {
    [0.995, 0.991, 0.990, 0.916, 0.936, 0.938, 0.058, 0.0301]
}

// ─── ValvePositions ──────────────────────────────────────────────────────────
// Snapshot das posições físicas dos atuadores lidas do vetor de estado.
// Construída a cada chamada de dynamics() a partir de state[38..50].

pub struct ValvePositions {
    pub a_feed: f64,            // XMV-01 state[38]: válvula feed A
    pub d_feed: f64,            // XMV-02 state[39]: válvula feed D
    pub e_feed: f64,            // XMV-03 state[40]: válvula feed E
    pub c_feed: f64,            // XMV-04 state[41]: válvula feed A/C
    pub recycle: f64,           // XMV-05 state[42]: anti-surge do compressor
    pub separator_liquid: f64,  // XMV-06 state[43]: líquido separador → stripper
    pub purge: f64,             // XMV-07 state[44]: purga
    pub product: f64,           // XMV-08 state[45]: produto (saída stripper)
    pub condenser_cooling: f64, // XMV-09 state[46]: água resfriamento condensador
    pub reactor_cooling: f64,   // XMV-10 state[47]: água resfriamento reator
    pub stripper_steam: f64,    // XMV-11 state[48]: vapor stripper
    pub agitator: f64,          // XMV-12 state[49]: agitador do reator
}

impl ValvePositions {
    pub fn from_state(state: &[f64]) -> Self {
        Self {
            a_feed:            state[38],
            d_feed:            state[39],
            e_feed:            state[40],
            c_feed:            state[41],
            recycle:           state[42],
            separator_liquid:  state[43],
            purge:             state[44],
            product:           state[45],
            condenser_cooling: state[46],
            reactor_cooling:   state[47],
            stripper_steam:    state[48],
            agitator:          state[49],
        }
    }
}

// ─── TennesseeEastmanModel ────────────────────────────────────────────────────

pub struct TennesseeEastmanModel {
    // Configuração e estado persistente da química
    pub constants: TepConstants,
    pub disturbance: TepDisturbanceState,
    pub idv: [i32; 20],
    pub idv_step_mag: [f64; 20],
    pub time: f64,
    pub xst: [[f64; 13]; 8],
    pub tst: [f64; 13],
    pub sfr: [f64; 8],
    pub xmeas: [f64; 41],

    // Atuadores — cada um é um DynamicModel com 1 estado próprio
    pub valves: [Valve; 11],
    pub agitator: Agitator,
}

impl TennesseeEastmanModel {
    pub fn new(initial: &InitialState) -> Self {
        let flat = initial.flatten();
        let initial_vpos: [f64; 11] = flat[38..49].try_into().unwrap();
        let initial_agitator_pos = flat[49];
        let valves = std::array::from_fn(|i| {
            let mut v = Valve::new(VTAU[i]);
            v.set_command(initial_vpos[i]);
            v
        });
        let mut agitator = Agitator::new(AGITATOR_TAU);
        agitator.set_command(initial_agitator_pos);
        Self {
            constants: TepConstants::new(),
            disturbance: TepDisturbanceState::new(),
            idv: [0; 20],
            idv_step_mag: {
                let mut m = [1.0_f64; 20];
                m[2] = 5.0;
                m[3] = 5.0;
                m[4] = 5.0;
                m
            },
            time: 0.0,
            xst: initial_xst(),
            tst: initial_tst(),
            sfr: initial_sfr(),
            xmeas: [0.0; 41],
            valves,
            agitator,
        }
    }

    pub fn set_commands(&mut self, xmv: &[f64]) {
        for (i, valve) in self.valves.iter_mut().enumerate() {
            if let Some(&cmd) = xmv.get(i) {
                valve.set_command(cmd);
            }
        }
        if let Some(&cmd) = xmv.get(11) {
            self.agitator.set_command(cmd);
        }
    }

    pub fn set_disturbances(&mut self, idv: &[i32]) {
        for (i, &v) in idv.iter().enumerate().take(20) {
            self.idv[i] = v;
        }
    }

    pub fn advance_time(&mut self, dt: f64) {
        self.time += dt;
    }

    pub fn xmeas(&self) -> &[f64; 41] {
        &self.xmeas
    }
}

impl DynamicModel for TennesseeEastmanModel {
    fn state_size(&self) -> usize {
        50 // 38 química + 11 válvulas + 1 agitador
    }

    fn dynamics(&mut self, state: &[f64]) -> Vec<f64> {
        let vpos = ValvePositions::from_state(state);
        let reactor_cw_return   = state[36];
        let separator_cw_return = state[37];

        // 1. Perturbações — atualiza feeds, retorna r1f/r2f/tcwr/tcws
        let dist = TepDisturbances::compute(
            self.time,
            &self.idv,
            &self.idv_step_mag,
            &mut self.disturbance.inner,
            &mut self.xst,
            &mut self.tst,
        );

        // 2. Termodinâmica de cada unidade (depende do estado e das constantes)
        let reactor = TepReactor::compute_thermo(
            &state[0..9],
            dist.reaction_factor_1,
            dist.reaction_factor_2,
            self.time,
            &self.constants,
        );
        let separator = TepSeparator::compute_thermo(&state[9..18], reactor.temperature, &self.constants);
        let stripper  = TepStripper::compute_thermo(&state[18..27], separator.temperature, &self.constants);
        let compressor = TepCompressor::compute_thermo(&state[27..36], separator.temperature, &self.constants);

        // 3. Fluxos (depende da termodinâmica + vpos das válvulas)
        let flows = TepFlows::compute(
            &reactor,                // pressões, temperaturas e frações do reator
            &separator,              // pressões, temperaturas e frações do separador
            &stripper,               // pressões, temperaturas e frações do stripper
            &compressor,             // pressões, temperaturas e frações do compressor
            &mut self.xst,           // composições dos streams (completadas aqui dentro)
            &mut self.tst,           // temperaturas dos streams (completadas aqui dentro)
            &mut self.sfr,           // fatores de split vapor/líquido do stripper
            &vpos,                   // posições dos atuadores (do vetor de estado)
            &self.idv,               // flags de distúrbio ativos (IDV 1–20)
            &self.constants,         // constantes termodinâmicas e ranges de válvula
            self.time,               // tempo atual (para distúrbios dinâmicos)
            &self.disturbance.inner, // estado dos distúrbios cúbicos (IDV 16/17/18)
        );

        // 4. Transferência de calor (depende dos fluxos + temperaturas de resfriamento)
        let heat = TepHeat::compute(
            &reactor,
            &stripper,
            &flows,
            reactor_cw_return,
            separator_cw_return,
            reactor.temperature,
            self.time,
            &self.disturbance.inner,
        );

        // 5. Medições + shutdown
        let shutdown = TepMeasurements::compute(
            &reactor,
            &separator,
            &stripper,
            &compressor,
            &flows,
            &heat,
            reactor_cw_return,
            separator_cw_return,
            &mut self.xmeas,
        );
        if shutdown {
            return vec![0.0; 50];
        }

        // 6. Derivadas da química [0..38]
        let mut yp = vec![0.0f64; 50];
        yp[0..9].copy_from_slice(&TepReactor::compute_derivatives(&flows, &reactor, heat.reactor_heat));
        yp[9..18].copy_from_slice(&TepSeparator::compute_derivatives(&flows, heat.separator_heat));
        yp[18..27].copy_from_slice(&TepStripper::compute_derivatives(&flows, heat.condenser_heat));
        yp[27..36].copy_from_slice(&TepCompressor::compute_derivatives(&flows));
        // yp[36], yp[37] (reactor_cw_return, separator_cw_return) — derivada zero (estados externos)

        // 7. Dinâmica dos atuadores — válvulas [38..49] e agitador [49]
        for (i, valve) in self.valves.iter_mut().enumerate() {
            yp[38 + i] = valve.dynamics(&state[38 + i..39 + i])[0];
        }
        yp[49] = self.agitator.dynamics(&state[49..50])[0];

        yp
    }

    fn name(&self) -> &'static str {
        "TennesseeEastmanModel"
    }
}
