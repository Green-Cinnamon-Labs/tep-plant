// dynamics/tep/plant_v2.rs
//
// TepPlantCore — composite DynamicModelV2.
//
// derivatives() orquestra os subsistemas em sequência:
//   TepDisturbances → TepReactor → TepSeparator → TepStripper → TepCompressor
//     → TepFlows → TepHeat → TepMeasurements → monta yp
//
// Cada subsistema recebe inputs explícitos e retorna um output struct.
// Nenhum subsistema conhece o vetor de estado completo — só seu slice.
//
// State layout (50 elements):
//   [0..9]   — reator (ucvr[0..3], uclr[3..8], etr)
//   [9..18]  — separador (ucvs[0..3], ucls[3..8], ets)
//   [18..27] — stripper (uclc[0..8], etc)
//   [27..36] — compressor/condensador (ucvv[0..8], etv)
//   [36]     — twr (temperatura de retorno do resfriamento do reator)
//   [37]     — tws (temperatura de retorno do separador)
//   [38..50] — vpos (posições das válvulas)

use crate::dynamics::model_v2::DynamicModelV2;
use crate::dynamics::tep::constants::TepConstants;
use crate::dynamics::tep::disturbance_state::TepDisturbanceState;
use crate::dynamics::tep::initial_state::InitialState;
use crate::dynamics::tep::subsystems::{
    TepCompressor, TepDisturbances, TepFlows, TepHeat, TepMeasurements, TepReactor, TepSeparator,
    TepStripper,
};

fn initial_xst() -> [[f64; 13]; 8] {
    let mut xst = [[0.0f64; 13]; 8];
    xst[1][0] = 0.0001; xst[3][0] = 0.9999;
    xst[4][1] = 0.9999; xst[5][1] = 0.0001;
    xst[0][2] = 0.9999; xst[1][2] = 0.0001;
    xst[0][3] = 0.485;  xst[1][3] = 0.005; xst[2][3] = 0.510;
    xst
}

fn initial_tst() -> [f64; 13] {
    let mut tst = [0.0f64; 13];
    tst[0] = 45.0; tst[1] = 45.0; tst[2] = 45.0; tst[3] = 45.0;
    tst
}

fn initial_sfr() -> [f64; 8] {
    [0.995, 0.991, 0.990, 0.916, 0.936, 0.938, 0.058, 0.0301]
}

const VTAU: [f64; 12] = [
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
    5.0 / 3600.0,
];

// ─── TepPlantCore ─────────────────────────────────────────────────────────────

pub struct TepPlantCore {
    // Configuração
    pub constants: TepConstants,
    pub disturbance: TepDisturbanceState,
    pub xmv: [f64; 12],
    pub idv: [i32; 20],
    pub idv_step_mag: [f64; 20],
    pub time: f64,

    // Estado persistente de correntes (colunas 0..4 são feeds; 4..13 são recalculadas a cada step)
    pub xst: [[f64; 13]; 8],
    pub tst: [f64; 13],
    pub sfr: [f64; 8],

    // Medições (snapshot sem ruído — preenchido por TepMeasurements)
    pub xmeas: [f64; 41],
}

impl TepPlantCore {
    pub fn new(initial: &InitialState) -> Self {
        let flat = initial.flatten();
        Self {
            constants: TepConstants::new(),
            disturbance: TepDisturbanceState::new(),
            xmv: flat[38..50].try_into().unwrap(),
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
        }
    }

    pub fn set_commands(&mut self, xmv: &[f64]) {
        for (i, &v) in xmv.iter().enumerate().take(12) {
            self.xmv[i] = v;
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

    pub fn raw_xmeas(&self) -> &[f64; 41] {
        &self.xmeas
    }
}

impl DynamicModelV2 for TepPlantCore {
    fn state_size(&self) -> usize {
        50
    }

    fn derivatives(&mut self, state: &[f64]) -> Vec<f64> {
        // Extrai entradas comuns do vetor de estado
        let vpos: [f64; 12] = state[38..50].try_into().unwrap();
        let twr = state[36];
        let tws = state[37];

        // ── 1. Perturbações (Blocks 7–12) ────────────────────────────────────
        let dist = TepDisturbances::compute(
            self.time,
            &self.idv,
            &self.idv_step_mag,
            &mut self.disturbance.inner,
            &mut self.xst,
            &mut self.tst,
        );

        // ── 2. Termodinâmica de cada unidade (Blocks 14–17) ──────────────────
        let rx = TepReactor::compute_thermo(
            &state[0..9],
            dist.r1f,
            dist.r2f,
            self.time,
            &self.constants,
        );
        let sep = TepSeparator::compute_thermo(&state[9..18], rx.tcr, &self.constants);
        let str = TepStripper::compute_thermo(&state[18..27], sep.tcs, &self.constants);
        let cmp = TepCompressor::compute_thermo(&state[27..36], sep.tcs, &self.constants);

        // ── 3. Fluxos e fracionamento (Blocks 19–31) ─────────────────────────
        let flows = TepFlows::compute(
            &rx,
            &sep,
            &str,
            &cmp,
            &mut self.xst,
            &mut self.tst,
            &mut self.sfr,
            &vpos,
            &self.idv,
            &self.constants,
            self.time,
            &self.disturbance.inner,
        );

        // ── 4. Transferência de calor (Blocks 32–34) ─────────────────────────
        let heat = TepHeat::compute(
            &rx,
            &str,
            &flows,
            twr,
            tws,
            rx.tcr,
            self.time,
            &self.disturbance.inner,
        );

        // ── 5. Medições + detecção de shutdown (Blocks 35–36) ────────────────
        let shutdown = TepMeasurements::compute(
            &rx,
            &sep,
            &str,
            &cmp,
            &flows,
            &heat,
            twr,
            tws,
            &mut self.xmeas,
        );

        if shutdown {
            return vec![0.0f64; 50];
        }

        // ── 6. Monta vetor de derivadas (Block 40) ───────────────────────────
        let mut yp = vec![0.0f64; 50];

        // Cada subsistema retorna seu slice de derivadas
        let dr = TepReactor::compute_derivatives(&flows, &rx, heat.qur);
        let ds = TepSeparator::compute_derivatives(&flows, heat.qus);
        let dst = TepStripper::compute_derivatives(&flows, heat.quc);
        let dc = TepCompressor::compute_derivatives(&flows);

        yp[0..9].copy_from_slice(&dr);
        yp[9..18].copy_from_slice(&ds);
        yp[18..27].copy_from_slice(&dst);
        yp[27..36].copy_from_slice(&dc);
        yp[36] = 0.0;
        yp[37] = 0.0;
        for i in 0..12 {
            yp[38 + i] = (self.xmv[i] - vpos[i]) / VTAU[i];
        }

        yp
    }

    fn name(&self) -> &'static str {
        "TepPlantCore"
    }
}
