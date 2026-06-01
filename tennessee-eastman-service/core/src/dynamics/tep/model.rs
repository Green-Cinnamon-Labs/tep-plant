// core/src/dynamics/tep/model.rs

use crate::state::State;
use crate::dynamics::model::DynamicModel;
use crate::snapshot::Alarm;
use crate::dynamics::tep::constants::TepConstants;
use crate::dynamics::tep::initial_state::InitialState;
use crate::dynamics::tep::disturbance_state::TepDisturbanceState;
use crate::dynamics::disturbance::{eval_disturbance, update_segment, white_noise};
use crate::dynamics::thermo::{mixture_enthalpy, temperature_from_enthalpy, liquid_density};

// Volume constants from TEINIT (teprob.f)
const VTR: f64 = 1300.0;   // reactor total volume
const VTS: f64 = 3500.0;   // separator total volume
const VTC: f64 = 156.5;    // stripper total volume
const VTV: f64 = 5000.0;   // compressor total volume

// Actuator range constants from TEINIT
const VRNG: [f64; 12] = [400.0, 400.0, 100.0, 1500.0, 0.0, 0.0, 1500.0, 1000.0, 0.03, 1000.0, 1200.0, 0.0];

// Compressor constants
const CPFLMX: f64 = 280275.0;
const CPPRMX: f64 = 1.3;

// Heat of reaction constants from TEINIT
const HTR: [f64; 2] = [0.06899381054, 0.05];

// Valve time constants from TEINIT (already converted to hours)
const VTAU: [f64; 12] = [
    8.0/3600.0, 8.0/3600.0, 6.0/3600.0, 9.0/3600.0,
    7.0/3600.0, 5.0/3600.0, 5.0/3600.0, 5.0/3600.0,
    120.0/3600.0, 5.0/3600.0, 5.0/3600.0, 5.0/3600.0,
];

// Noise standard deviations from TEINIT
const XNS: [f64; 41] = [
    0.0012, 18.0, 22.0, 0.05, 0.2, 0.21, 0.3, 0.5, 0.01, 0.0017,
    0.01, 1.0, 0.3, 0.125, 1.0, 0.3, 0.115, 0.01, 1.15, 0.2,
    0.01, 0.01, 0.25, 0.1, 0.25, 0.1, 0.25, 0.025, 0.25, 0.1,
    0.25, 0.1, 0.25, 0.025, 0.05, 0.05, 0.01, 0.01, 0.01, 0.5, 0.5,
];

// Initial stream compositions from TEINIT (8 components x 13 streams, row-major)
// xst[component][stream], 0-indexed
fn initial_xst() -> [[f64; 13]; 8] {
    let mut xst = [[0.0f64; 13]; 8];
    // Stream 1 (index 0): pure D feed
    xst[1][0] = 0.0001; xst[3][0] = 0.9999;
    // Stream 2 (index 1): pure E feed
    xst[4][1] = 0.9999; xst[5][1] = 0.0001;
    // Stream 3 (index 2): pure A feed
    xst[0][2] = 0.9999; xst[1][2] = 0.0001;
    // Stream 4 (index 3): A/C feed (updated each step via disturbance)
    xst[0][3] = 0.485; xst[1][3] = 0.005; xst[2][3] = 0.510;
    // Stream temperatures initialized elsewhere (TST)
    xst
}

// Initial stream temperatures from TEINIT
fn initial_tst() -> [f64; 13] {
    let mut tst = [0.0f64; 13];
    tst[0] = 45.0; tst[1] = 45.0; tst[2] = 45.0; tst[3] = 45.0;
    tst
}

// Split fractions from TEINIT
fn initial_sfr() -> [f64; 8] {
    [0.995, 0.991, 0.990, 0.916, 0.936, 0.938, 0.058, 0.0301]
}

/// Extract a column from the stream composition matrix as a fixed-size array.
/// Equivalent to XST(1:8, stream) in FORTRAN — 0-indexed.
fn xst_col(xst: &[[f64; 13]; 8], stream: usize) -> [f64; 8] {
    let mut arr = [0.0f64; 8];
    for i in 0..8 {
        arr[i] = xst[i][stream];
    }
    arr
}

pub struct TennesseeEastmanModel {
    pub constants:   TepConstants,
    pub disturbance: TepDisturbanceState,
    pub xmv:          [f64; 12],  // valve commands (external)
    pub idv:          [i32; 20],  // disturbance flags (external)
    pub idv_step_mag: [f64; 20],  // magnitude por IDV step (°C ou fração); default 5.0 para IDV 3/4/5
    pub time:        f64,        // current simulation time (hours)
    // Sampler state (TGAS, TPROD, XDEL from TEPROC)
    tgas:  f64,
    tprod: f64,
    xdel:  [f64; 41],
    xmeas: [f64; 41],
    // Walking disturbance factors (R1F, R2F initialized to 1.0)
    r1f: f64,
    r2f: f64,
    // Cooling water temperatures (TCWR, TCWS)
    tcwr: f64,
    tcws: f64,
    // Stream compositions and temperatures
    xst:  [[f64; 13]; 8],
    tst:  [f64; 13],
    sfr:  [f64; 8],
}

impl TennesseeEastmanModel {
    pub fn new(initial: &InitialState) -> Self {
        let flat = initial.flatten();
        Self {
            constants:   TepConstants::new(),
            disturbance: TepDisturbanceState::new(),
            xmv:         flat[38..50].try_into().unwrap(),
            idv:         [0; 20],
            idv_step_mag: {
                let mut m = [1.0_f64; 20];
                m[2] = 5.0; // IDV(3): D feed temperature +5°C
                m[3] = 5.0; // IDV(4): reactor CW inlet +5°C
                m[4] = 5.0; // IDV(5): condenser CW inlet +5°C
                m
            },
            time:        0.0,
            tgas:        0.1,
            tprod:       0.25,
            xdel:        [0.0; 41],
            xmeas:       [0.0; 41],
            r1f:         1.0,
            r2f:         1.0,
            tcwr:        eval_disturbance(4, 0.0, &TepDisturbanceState::new().inner),
            tcws:        eval_disturbance(5, 0.0, &TepDisturbanceState::new().inner),
            xst:         initial_xst(),
            tst:         initial_tst(),
            sfr:         initial_sfr(),
        }
    }

    /// Returns the last computed XMEAS vector (41 measurements)
    pub fn xmeas(&self) -> &[f64; 41] {
        &self.xmeas
    }
}

impl DynamicModel for TennesseeEastmanModel {

    fn measurements(&self) -> &[f64] {
        self.xmeas()
    }

    fn get_mv(&self) -> Vec<f64> {
        self.xmv.to_vec()
    }

    fn set_inputs(&mut self, mv: &[f64], dv: &[f64]) {
        for (i, &v) in mv.iter().enumerate().take(12) {
            self.xmv[i] = v;
        }
        for (i, &v) in dv.iter().enumerate().take(20) {
            self.idv[i] = if v != 0.0 { 1 } else { 0 };
        }
    }

    fn advance_time(&mut self, dt: f64) {
        self.time += dt;
    }

    fn set_idv_step_mag(&mut self, idx: usize, mag: f64) {
        if idx < 20 {
            self.idv_step_mag[idx] = mag;
        }
    }

    fn alarms(&self) -> Vec<Alarm> {
        let x = &self.xmeas;
        vec![
            Alarm { name: "Reactor P high (>3000 kPa)",  active: x[6]  > 3000.0 },
            Alarm { name: "Reactor T high (>175 °C)",    active: x[8]  > 175.0  },
            Alarm { name: "Reactor Lv high (>90%)",      active: x[7]  > 90.0   },
            Alarm { name: "Reactor Lv low (<10%)",       active: x[7]  < 10.0   },
            Alarm { name: "Sep Lv high (>90%)",          active: x[11] > 90.0   },
            Alarm { name: "Sep Lv low (<10%)",           active: x[11] < 10.0   },
            Alarm { name: "Stripper Lv high (>90%)",     active: x[14] > 90.0   },
            Alarm { name: "Stripper Lv low (<10%)",      active: x[14] < 10.0   },
        ]
    }

    fn derivatives(&mut self, state: &State) -> Vec<f64> {
        let yy = &state.x;
        let c  = &self.constants;
        let ds = &mut self.disturbance.inner;
        let time = self.time;

        // --------------------------------------------------------
        // Block 7: normalize disturbance flags
        // --------------------------------------------------------
        let mut idv = self.idv;
        for i in 0..20 {
            idv[i] = if idv[i] > 0 { 1 } else { 0 };
        }

        // --------------------------------------------------------
        // Block 8: map IDV to walking disturbance channels
        // --------------------------------------------------------
        ds.channels[0].active  = idv[7];
        ds.channels[1].active  = idv[7];
        ds.channels[2].active  = idv[8];
        ds.channels[3].active  = idv[9];
        ds.channels[4].active  = idv[10];
        ds.channels[5].active  = idv[11];
        ds.channels[6].active  = idv[12];
        ds.channels[7].active  = idv[12];
        ds.channels[8].active  = idv[15];
        ds.channels[9].active  = idv[16];
        ds.channels[10].active = idv[17];
        ds.channels[11].active = idv[19];

        // --------------------------------------------------------
        // Block 9: update walking disturbance segments (channels 0-8)
        // --------------------------------------------------------
        for i in 0..9 {
            if time >= ds.channels[i].t_next {
                let hw = ds.channels[i].t_next - ds.channels[i].t_last;
                let sw   = ds.channels[i].a + hw * (ds.channels[i].b + hw * (ds.channels[i].c + hw * ds.channels[i].d));
                let spw  = ds.channels[i].b + hw * (2.0 * ds.channels[i].c + 3.0 * hw * ds.channels[i].d);
                ds.channels[i].t_last = ds.channels[i].t_next;
                update_segment(i, sw, spw, ds);
            }
        }

        // --------------------------------------------------------
        // Block 10: special sticking valve disturbances (channels 9-11)
        // --------------------------------------------------------
        for i in 9..12 {
            if time >= ds.channels[i].t_next {
                let hw = ds.channels[i].t_next - ds.channels[i].t_last;
                let sw  = ds.channels[i].a + hw * (ds.channels[i].b + hw * (ds.channels[i].c + hw * ds.channels[i].d));
                let spw = ds.channels[i].b + hw * (2.0 * ds.channels[i].c + 3.0 * hw * ds.channels[i].d);
                ds.channels[i].t_last = ds.channels[i].t_next;
                if sw > 0.1 {
                    ds.channels[i].a = sw;
                    ds.channels[i].b = spw;
                    ds.channels[i].c = -(3.0 * sw + 0.2 * spw) / 0.01;
                    ds.channels[i].d =  (2.0 * sw + 0.1 * spw) / 0.001;
                    ds.channels[i].t_next = ds.channels[i].t_last + 0.1;
                } else {
                    use crate::dynamics::disturbance::lcg_rand;
                    let hw2 = ds.channels[i].h_span * lcg_rand(-1, ds) + ds.channels[i].h_zero;
                    ds.channels[i].a = 0.0;
                    ds.channels[i].b = 0.0;
                    ds.channels[i].c = ds.channels[i].active as f64 / (hw2 * hw2);
                    ds.channels[i].d = 0.0;
                    ds.channels[i].t_next = ds.channels[i].t_last + hw2;
                }
            }
        }

        // --------------------------------------------------------
        // Block 11: initialize disturbance profiles at time == 0
        // --------------------------------------------------------
        if time == 0.0 {
            for i in 0..12 {
                ds.channels[i].a      = ds.channels[i].s_zero;
                ds.channels[i].b      = 0.0;
                ds.channels[i].c      = 0.0;
                ds.channels[i].d      = 0.0;
                ds.channels[i].t_last = 0.0;
                ds.channels[i].t_next = 0.1;
            }
        }

        // --------------------------------------------------------
        // Block 12: update feed compositions and temperatures
        // --------------------------------------------------------
        self.xst[0][3] = eval_disturbance(0, time, ds) - idv[0] as f64 * 0.03 - idv[1] as f64 * 2.43719e-3;
        self.xst[1][3] = eval_disturbance(1, time, ds) + idv[1] as f64 * 0.005;
        self.xst[2][3] = 1.0 - self.xst[0][3] - self.xst[1][3];

        self.tst[0]  = eval_disturbance(2, time, ds) + idv[2] as f64 * self.idv_step_mag[2];
        self.tst[3]  = eval_disturbance(3, time, ds);
        self.tcwr    = eval_disturbance(4, time, ds) + idv[3] as f64 * self.idv_step_mag[3];
        self.tcws    = eval_disturbance(5, time, ds) + idv[4] as f64 * self.idv_step_mag[4];
        self.r1f     = eval_disturbance(6, time, ds);
        self.r2f     = eval_disturbance(7, time, ds);

        let r1f = self.r1f;
        let r2f = self.r2f;

        // --------------------------------------------------------
        // Block 13: unpack state vector YY into process variables
        // --------------------------------------------------------
        let mut ucvr = [0.0f64; 8];
        let mut ucvs = [0.0f64; 8];
        let mut uclr = [0.0f64; 8];
        let mut ucls = [0.0f64; 8];
        let mut uclc = [0.0f64; 8];
        let mut ucvv = [0.0f64; 8];

        // Components 1-3 (idx 0-2): vapor only in reactor/separator
        for i in 0..3 {
            ucvr[i] = yy[i];
            ucvs[i] = yy[i + 9];
        }
        // Components 4-8 (idx 3-7): liquid only in reactor/separator
        for i in 3..8 {
            uclr[i] = yy[i];
            ucls[i] = yy[i + 9];
        }
        // Stripper and compressor
        for i in 0..8 {
            uclc[i] = yy[i + 18];
            ucvv[i] = yy[i + 27];
        }

        let etr = yy[8];
        let ets = yy[17];
        let etc = yy[26];
        let etv = yy[35];
        let _twr = yy[36]; // historical state; twr recomputed quasi-statically in Block 32
        let tws = yy[37];

        // Valve positions (states 39-50, idx 38-49)
        let mut vpos = [0.0f64; 12];
        for i in 0..12 {
            vpos[i] = yy[i + 38];
        }

        // --------------------------------------------------------
        // Block 14: molar totals and mole fractions
        // --------------------------------------------------------
        let utlr: f64 = uclr.iter().sum();
        let utls: f64 = ucls.iter().sum();
        let utlc: f64 = uclc.iter().sum();
        let utvv: f64 = ucvv.iter().sum();

        let mut xlr = [0.0f64; 8];
        let mut xls = [0.0f64; 8];
        let mut xlc = [0.0f64; 8];
        let mut xvv = [0.0f64; 8];
        for i in 0..8 {
            xlr[i] = uclr[i] / utlr;
            xls[i] = ucls[i] / utls;
            xlc[i] = uclc[i] / utlc;
            xvv[i] = ucvv[i] / utvv;
        }

        // --------------------------------------------------------
        // Block 15: specific energy and temperatures
        // --------------------------------------------------------
        let esr = etr / utlr;
        let ess = ets / utls;
        let esc = etc / utlc;
        let esv = etv / utvv;

        let tcr_init = if time == 0.0 { 120.0 } else { 0.0 };
        let tcr = temperature_from_enthalpy(&xlr, tcr_init, esr, 0, c);
        let tkr = tcr + 273.15;

        let tcs = temperature_from_enthalpy(&xls, tcr, ess, 0, c);
        let tks = tcs + 273.15;

        let tcc = temperature_from_enthalpy(&xlc, tcs, esc, 0, c);
        let tcv = temperature_from_enthalpy(&xvv, tcs, esv, 2, c);
        let tkv = tcv + 273.15;

        // --------------------------------------------------------
        // Block 16: densities, volumes, pressures
        // --------------------------------------------------------
        let dlr = liquid_density(&xlr, tcr, c);
        let dls = liquid_density(&xls, tcs, c);
        let dlc = liquid_density(&xlc, tcc, c);

        let vlr = utlr / dlr;
        let vls = utls / dls;
        let vlc = utlc / dlc;

        let vvr = VTR - vlr;
        let vvs = VTS - vls;

        const RG: f64 = 998.9;
        let mut ppr = [0.0f64; 8];
        let mut pps = [0.0f64; 8];
        let mut ptr = 0.0f64;
        let mut pts = 0.0f64;

        // Non-condensable gases (ideal gas law)
        for i in 0..3 {
            ppr[i] = ucvr[i] * RG * tkr / vvr;
            ptr    += ppr[i];
            pps[i] = ucvs[i] * RG * tks / vvs;
            pts    += pps[i];
        }
        // Condensable components (Antoine equation)
        for i in 3..8 {
            let vpr = (c.avp[i] + c.bvp[i] / (tcr + c.cvp[i])).exp();
            ppr[i] = vpr * xlr[i];
            ptr   += ppr[i];

            let vpr = (c.avp[i] + c.bvp[i] / (tcs + c.cvp[i])).exp();
            pps[i] = vpr * xls[i];
            pts   += pps[i];
        }

        let ptv = utvv * RG * tkv / VTV;




        // --------------------------------------------------------
        // Block 17: reaction kinetics (Arrhenius)
        // --------------------------------------------------------
        let mut xvr = [0.0f64; 8];
        let mut xvs = [0.0f64; 8];
        for i in 0..8 {
            xvr[i] = ppr[i] / ptr;
            xvs[i] = pps[i] / pts;
        }

        let utvr = ptr * vvr / RG / tkr;
        let utvs = pts * vvs / RG / tks;

        let mut ucvr = ucvr; // shadow to allow mutation
        let mut ucvs = ucvs;
        for i in 3..8 {
            ucvr[i] = utvr * xvr[i];
            ucvs[i] = utvs * xvs[i];
        }

        let mut rr = [0.0f64; 4];
        rr[0] = (31.5859536 - 40000.0 / 1.987 / tkr).exp() * r1f;
        rr[1] = (3.00094014 - 20000.0 / 1.987 / tkr).exp() * r2f;
        rr[2] = (53.4060443 - 60000.0 / 1.987 / tkr).exp();
        rr[3] = rr[2] * 0.767488334;

        if ppr[0] > 0.0 && ppr[2] > 0.0 {
            let rf1 = ppr[0].powf(1.1544);
            let rf2 = ppr[2].powf(0.3735);
            rr[0] *= rf1 * rf2 * ppr[3];
            rr[1] *= rf1 * rf2 * ppr[4];
        } else {
            rr[0] = 0.0;
            rr[1] = 0.0;
        }
        rr[2] *= ppr[0] * ppr[4];
        rr[3] *= ppr[0] * ppr[3];

        for r in rr.iter_mut() {
            *r *= vvr;
        }

        // --------------------------------------------------------
        // Block 18: component reaction rates
        // --------------------------------------------------------
        let mut crxr = [0.0f64; 8];
        crxr[0] = -rr[0] - rr[1] - rr[2];
        crxr[2] = -rr[0] - rr[1];
        crxr[3] = -rr[0] - 1.5 * rr[3];
        crxr[4] = -rr[1] - rr[2];
        crxr[5] =  rr[2] + rr[3];
        crxr[6] =  rr[0];
        crxr[7] =  rr[1];

        let rh = rr[0] * HTR[0] + rr[1] * HTR[1];

        // --------------------------------------------------------
        // Block 19: mean molecular weights of streams
        // --------------------------------------------------------
        let xst = &mut self.xst;
        let tst = &mut self.tst;
        let sfr = &mut self.sfr;

        for i in 0..8 {
            xst[i][5]  = xvv[i];
            xst[i][7]  = xvr[i];
            xst[i][8]  = xvs[i];
            xst[i][9]  = xvs[i];
            xst[i][10] = xls[i];
            xst[i][12] = xlc[i];
        }

        let mut xmws = [0.0f64; 13];
        for i in 0..8 {
            xmws[0]  += xst[i][0]  * c.xmw[i];
            xmws[1]  += xst[i][1]  * c.xmw[i];
            xmws[5]  += xst[i][5]  * c.xmw[i];
            xmws[7]  += xst[i][7]  * c.xmw[i];
            xmws[8]  += xst[i][8]  * c.xmw[i];
            xmws[9]  += xst[i][9]  * c.xmw[i];
        }

        // --------------------------------------------------------
        // Block 20: stream temperatures
        // --------------------------------------------------------
        tst[5]  = tcv;
        tst[7]  = tcr;
        tst[8]  = tcs;
        tst[9]  = tcs;
        tst[10] = tcs;
        tst[12] = tcc;

        // --------------------------------------------------------
        // Block 21: stream enthalpies
        // --------------------------------------------------------
        let mut hst = [0.0f64; 13];
        hst[0]  = mixture_enthalpy(&xst_col(xst, 0),  tst[0],  1, c);
        hst[1]  = mixture_enthalpy(&xst_col(xst, 1),  tst[1],  1, c);
        hst[2]  = mixture_enthalpy(&xst_col(xst, 2),  tst[2],  1, c);
        hst[3]  = mixture_enthalpy(&xst_col(xst, 3),  tst[3],  1, c);
        hst[5]  = mixture_enthalpy(&xst_col(xst, 5),  tst[5],  1, c);
        hst[7]  = mixture_enthalpy(&xst_col(xst, 7),  tst[7],  1, c);
        hst[8]  = mixture_enthalpy(&xst_col(xst, 8),  tst[8],  1, c);
        hst[9]  = hst[8];
        hst[10] = mixture_enthalpy(&xst_col(xst, 10), tst[10], 0, c);
        hst[12] = mixture_enthalpy(&xst_col(xst, 12), tst[12], 0, c);

        // --------------------------------------------------------
        // Block 22: valve-driven flow rates
        // --------------------------------------------------------
        let mut ftm = [0.0f64; 13];
        ftm[0]  = vpos[0] * VRNG[0] / 100.0;
        ftm[1]  = vpos[1] * VRNG[1] / 100.0;
        ftm[2]  = vpos[2] * (1.0 - idv[5] as f64) * VRNG[2] / 100.0;
        ftm[3]  = vpos[3] * (1.0 - idv[6] as f64 * 0.2) * VRNG[3] / 100.0 + 1e-10;
        ftm[10] = vpos[6] * VRNG[6] / 100.0;
        ftm[12] = vpos[7] * VRNG[7] / 100.0;

        let uac = vpos[8] * VRNG[8] * (1.0 + eval_disturbance(8, time, ds)) / 100.0;
        let agsp = (vpos[11] + 150.0) / 100.0;

        // --------------------------------------------------------
        // Block 23: pressure-driven flow rates
        // --------------------------------------------------------
        let dlp = (ptv - ptr).max(0.0);
        let flms = 1937.6 * dlp.sqrt();
        ftm[5] = flms / xmws[5];

        let dlp = (ptr - pts).max(0.0);
        let flms = 4574.21 * dlp.sqrt() * (1.0 - 0.25 * eval_disturbance(11, time, ds));
        ftm[7] = flms / xmws[7];

        let dlp = (pts - 760.0).max(0.0);
        let flms = vpos[5] * 0.151169 * dlp.sqrt();
        ftm[9] = flms / xmws[9];

        // --------------------------------------------------------
        // Block 24: compressor and heat
        // --------------------------------------------------------
        let pr = (ptv / pts).max(1.0).min(CPPRMX);
        let flcoef = CPFLMX / 1.197;
        let mut flms = CPFLMX + flcoef * (1.0 - pr.powi(3));

        let cpdh = flms * (tcs + 273.15) * 1.8e-6 * 1.9872 * (ptv - pts) / (xmws[8] * pts);

        let dlp = (ptv - pts).max(0.0);
        flms -= vpos[4] * 53.349 * dlp.sqrt();
        flms  = flms.max(1e-3);

        ftm[8]  = flms / xmws[8];
        hst[8] += cpdh / ftm[8];

        // --------------------------------------------------------
        // Block 25: component flow matrix FCM
        // --------------------------------------------------------
        let mut fcm = [[0.0f64; 13]; 8];
        for i in 0..8 {
            fcm[i][0]  = xst[i][0]  * ftm[0];
            fcm[i][1]  = xst[i][1]  * ftm[1];
            fcm[i][2]  = xst[i][2]  * ftm[2];
            fcm[i][3]  = xst[i][3]  * ftm[3];
            fcm[i][5]  = xst[i][5]  * ftm[5];
            fcm[i][7]  = xst[i][7]  * ftm[7];
            fcm[i][8]  = xst[i][8]  * ftm[8];
            fcm[i][9]  = xst[i][9]  * ftm[9];
            fcm[i][10] = xst[i][10] * ftm[10];
            fcm[i][12] = xst[i][12] * ftm[12];
        }

        // --------------------------------------------------------
        // Block 26: stripper flash model
        // --------------------------------------------------------
        if ftm[10] > 0.1 {
            let tmpfac = if tcc > 170.0 {
                tcc - 120.262
            } else if tcc < 5.292 {
                0.1
            } else {
                363.744 / (177.0 - tcc) - 2.22579488
            };
            let vovrl = ftm[3] / ftm[10] * tmpfac;
            sfr[3] = 8.5010  * vovrl / (1.0 + 8.5010  * vovrl);
            sfr[4] = 11.402  * vovrl / (1.0 + 11.402  * vovrl);
            sfr[5] = 11.795  * vovrl / (1.0 + 11.795  * vovrl);
            sfr[6] = 0.0480  * vovrl / (1.0 + 0.0480  * vovrl);
            sfr[7] = 0.0242  * vovrl / (1.0 + 0.0242  * vovrl);
        } else {
            sfr[3] = 0.9999;
            sfr[4] = 0.999;
            sfr[5] = 0.999;
            sfr[6] = 0.99;
            sfr[7] = 0.98;
        }

        // --------------------------------------------------------
        // Block 27: stripper inlet flows
        // --------------------------------------------------------
        let mut fin = [0.0f64; 8];
        for i in 0..8 {
            fin[i] = fcm[i][3] + fcm[i][10];
        }

        // --------------------------------------------------------
        // Block 28: vapor/liquid split
        // --------------------------------------------------------
        ftm[4]  = 0.0;
        ftm[11] = 0.0;
        for i in 0..8 {
            fcm[i][4]  = sfr[i] * fin[i];
            fcm[i][11] = fin[i] - fcm[i][4];
            ftm[4]    += fcm[i][4];
            ftm[11]   += fcm[i][11];
        }

        // --------------------------------------------------------
        // Block 29: stream compositions after split
        // --------------------------------------------------------
        for i in 0..8 {
            xst[i][4]  = fcm[i][4]  / ftm[4];
            xst[i][11] = fcm[i][11] / ftm[11];
        }

        // --------------------------------------------------------
        // Block 30: stripper stream enthalpies
        // --------------------------------------------------------
        tst[4]  = tcc;
        tst[11] = tcc;
        hst[4]  = mixture_enthalpy(&xst_col(xst, 4),  tst[4],  1, c);
        hst[11] = mixture_enthalpy(&xst_col(xst, 11), tst[11], 0, c);

        // --------------------------------------------------------
        // Block 31: bypass / internal recycle
        // --------------------------------------------------------
        ftm[6] = ftm[5];
        hst[6] = hst[5];
        tst[6] = tst[5];
        for i in 0..8 {
            xst[i][6] = xst[i][5];
            fcm[i][6] = fcm[i][5];
        }

        // --------------------------------------------------------
        // Block 32: reactor heat exchange
        // --------------------------------------------------------
        let uarlev = if vlr / 7.8 > 50.0 {
            1.0
        } else if vlr / 7.8 < 10.0 {
            0.0
        } else {
            0.025 * vlr / 7.8 - 0.25
        };
        let uar = uarlev * (-0.5 * agsp * agsp + 2.75 * agsp - 2.5) * 855490e-6;
        // 2026-06-01: comentado para isolar hipótese de instabilidade no cold-start (Exp 14).
        // O modelo quasi-estático abaixo fazia twr responder a tcwr e tcr, mas quando tcr cai
        // abaixo de ~67°C o balanço produz twr < tcr → QUR < 0 (trocador "aquece" o reator),
        // comportamento ausente no FORTRAN original onde YP(37) nunca é atribuído e TWR fica
        // congelado em yy[36] = 94.6°C. Reativar e comparar após confirmar resultado do teste.
        //
        // let fcwr = vpos[9] * VRNG[9] * 0.001;
        // let cw_cap_r = fcwr * 0.00942_f64 + uar;
        // let twr = if cw_cap_r > 1e-12 {
        //     (fcwr * 0.00942_f64 * self.tcwr + uar * tcr) / cw_cap_r
        // } else {
        //     yy[36]
        // };
        let twr = yy[36]; // fiel ao FORTRAN original: TWR = YY(37), derivada nula
        let qur = uar * (twr - tcr) * (1.0 - 0.35 * eval_disturbance(9, time, ds));

        // --------------------------------------------------------
        // Block 33: separator heat exchange
        // --------------------------------------------------------
        let uas = 0.404655 * (1.0 - 1.0 / (1.0 + (ftm[7] / 3528.73).powi(4)));
        let qus = uas * (tws - tst[7]) * (1.0 - 0.25 * eval_disturbance(10, time, ds));

        // --------------------------------------------------------
        // Block 34: condenser cooling
        // --------------------------------------------------------
        let quc = if tcc < 100.0 { uac * (100.0 - tcc) } else { 0.0 };

        // --------------------------------------------------------
        // Block 35: build XMEAS
        // --------------------------------------------------------
        let xmeas = &mut self.xmeas;
        xmeas[0]  = ftm[2]  * 0.359 / 35.3145;
        xmeas[1]  = ftm[0]  * xmws[0] * 0.454;
        xmeas[2]  = ftm[1]  * xmws[1] * 0.454;
        xmeas[3]  = ftm[3]  * 0.359 / 35.3145;
        xmeas[4]  = ftm[8]  * 0.359 / 35.3145;
        xmeas[5]  = ftm[5]  * 0.359 / 35.3145;
        xmeas[6]  = (ptr - 760.0) / 760.0 * 101.325;
        xmeas[7]  = (vlr - 84.6) / 666.7 * 100.0;
        xmeas[8]  = tcr;
        xmeas[9]  = ftm[9]  * 0.359 / 35.3145;
        xmeas[10] = tcs;
        xmeas[11] = (vls - 27.5) / 290.0 * 100.0;
        xmeas[12] = (pts - 760.0) / 760.0 * 101.325;
        xmeas[13] = ftm[10] / dls / 35.3145;
        xmeas[14] = (vlc - 78.25) / VTC * 100.0;
        xmeas[15] = (ptv - 760.0) / 760.0 * 101.325;
        xmeas[16] = ftm[12] / dlc / 35.3145;
        xmeas[17] = tcc;
        xmeas[18] = quc * 1.04e3 * 0.454;
        xmeas[19] = cpdh * 0.29307e3;
        xmeas[20] = twr;
        xmeas[21] = tws;

        // --------------------------------------------------------
        // Block 36: shutdown detection
        // --------------------------------------------------------
        let isd = xmeas[6] > 3000.0
            || vlr / 35.3145 > 24.0
            || vlr / 35.3145 < 2.0
            || xmeas[8] > 175.0
            || vls / 35.3145 > 12.0
            || vls / 35.3145 < 1.0
            || vlc / 35.3145 > 8.0
            || vlc / 35.3145 < 1.0;

        // --------------------------------------------------------
        // Block 37: add measurement noise
        // --------------------------------------------------------
        if time > 0.0 && !isd {
            let ds = &mut self.disturbance.inner;
            for i in 0..22 {
                xmeas[i] += white_noise(XNS[i], ds);
            }
        }

        // --------------------------------------------------------
        // Block 38-39: sampled analyzers (composition measurements)
        // --------------------------------------------------------
        let xst = &self.xst;
        let mut xcmp = [0.0f64; 41];
        xcmp[22] = xst[0][6]  * 100.0;
        xcmp[23] = xst[1][6]  * 100.0;
        xcmp[24] = xst[2][6]  * 100.0;
        xcmp[25] = xst[3][6]  * 100.0;
        xcmp[26] = xst[4][6]  * 100.0;
        xcmp[27] = xst[5][6]  * 100.0;
        xcmp[28] = xst[0][9]  * 100.0;
        xcmp[29] = xst[1][9]  * 100.0;
        xcmp[30] = xst[2][9]  * 100.0;
        xcmp[31] = xst[3][9]  * 100.0;
        xcmp[32] = xst[4][9]  * 100.0;
        xcmp[33] = xst[5][9]  * 100.0;
        xcmp[34] = xst[6][9]  * 100.0;
        xcmp[35] = xst[7][9]  * 100.0;
        xcmp[36] = xst[3][12] * 100.0;
        xcmp[37] = xst[4][12] * 100.0;
        xcmp[38] = xst[5][12] * 100.0;
        xcmp[39] = xst[6][12] * 100.0;
        xcmp[40] = xst[7][12] * 100.0;

        if time == 0.0 {
            for i in 22..41 {
                self.xdel[i]  = xcmp[i];
                self.xmeas[i] = xcmp[i];
            }
            self.tgas  = 0.1;
            self.tprod = 0.25;
        }

        let ds = &mut self.disturbance.inner;
        if time >= self.tgas {
            for i in 22..36 {
                self.xmeas[i]  = self.xdel[i] + white_noise(XNS[i], ds);
                self.xdel[i]   = xcmp[i];
            }
            self.tgas += 0.1;
        }
        if time >= self.tprod {
            for i in 36..41 {
                self.xmeas[i]  = self.xdel[i] + white_noise(XNS[i], ds);
                self.xdel[i]   = xcmp[i];
            }
            self.tprod += 0.25;
        }

        // --------------------------------------------------------
        // Block 40: ODEs — state derivatives
        // --------------------------------------------------------
        let mut yp = vec![0.0f64; 50];

        for i in 0..8 {
            // Reactor: mass balance per component
            yp[i]      = fcm[i][6] - fcm[i][7] + crxr[i];
            // Separator
            yp[i + 9]  = fcm[i][7] - fcm[i][8] - fcm[i][9] - fcm[i][10];
            // Stripper
            yp[i + 18] = fcm[i][11] - fcm[i][12];
            // Compressor/VV
            yp[i + 27] = fcm[i][0] + fcm[i][1] + fcm[i][2] + fcm[i][4] + fcm[i][8] - fcm[i][5];
        }

        // Energy balances
        yp[8]  = hst[6] * ftm[6] - hst[7] * ftm[7] + rh + qur;
        yp[17] = hst[7] * ftm[7] - hst[8] * ftm[8]
               - hst[9] * ftm[9] - hst[10] * ftm[10] + qus;
        yp[26] = hst[3] * ftm[3] + hst[10] * ftm[10]
               - hst[4] * ftm[4] - hst[12] * ftm[12] + quc;
        yp[35] = hst[0] * ftm[0] + hst[1] * ftm[1]
               + hst[2] * ftm[2] + hst[4] * ftm[4]
               + hst[8] * ftm[8] - hst[5] * ftm[5];

        // twr resolved quasi-statically in Block 32; tws kept at snapshot value
        yp[36] = 0.0;
        yp[37] = 0.0;

        // Valve dynamics: dVPOS/dt = (XMV - VPOS) / VTAU
        for i in 0..12 {
            yp[i + 38] = (self.xmv[i] - vpos[i]) / VTAU[i];
        }

        // --------------------------------------------------------
        // Block 41: zero derivatives on shutdown
        // --------------------------------------------------------
        if isd {
            return vec![0.0f64; 50];
        }

        yp
    }
}