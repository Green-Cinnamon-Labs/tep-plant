// dynamics/tep/subsystems.rs
//
// Subsistemas da planta TEP. Cada struct é uma unidade de processo.
// Não implementam DynamicModelV2 — recebem inputs explícitos e retornam outputs.
// O orquestrador (TepPlantCore) chama cada um em sequência e injeta outputs
// de um como inputs do próximo.

use crate::dynamics::disturbance::{
    eval_disturbance, lcg_rand, update_segment, CubicDisturbanceState,
};
use crate::dynamics::tep::constants::TepConstants;
use crate::dynamics::thermo::{liquid_density, mixture_enthalpy, temperature_from_enthalpy};

// ─── Constantes físicas ───────────────────────────────────────────────────────
const VTR: f64 = 1300.0;
const VTS: f64 = 3500.0;
const VTC: f64 = 156.5;
const VTV: f64 = 5000.0;
const RG: f64 = 998.9;
const VRNG: [f64; 12] = [
    400.0, 400.0, 100.0, 1500.0, 0.0, 0.0, 1500.0, 1000.0, 0.03, 1000.0, 1200.0, 0.0,
];
const CPFLMX: f64 = 280275.0;
const CPPRMX: f64 = 1.3;
const HTR: [f64; 2] = [0.06899381054, 0.05];

fn xst_col(xst: &[[f64; 13]; 8], s: usize) -> [f64; 8] {
    let mut a = [0.0f64; 8];
    for i in 0..8 {
        a[i] = xst[i][s];
    }
    a
}

// ─── Output structs ───────────────────────────────────────────────────────────

pub struct DistOut {
    pub r1f: f64,
    pub r2f: f64,
    pub tcwr: f64,
    pub tcws: f64,
}

pub struct ReactorOut {
    pub tcr: f64,
    pub tkr: f64,
    pub ptr: f64,
    pub xlr: [f64; 8],
    pub vlr: f64,
    pub dlr: f64,
    pub vvr: f64,
    pub xvr: [f64; 8],
    pub ucvr: [f64; 8],
    pub utvr: f64,
    pub crxr: [f64; 8],
    pub rh: f64,
}

pub struct SepOut {
    pub tcs: f64,
    pub pts: f64,
    pub xls: [f64; 8],
    pub vls: f64,
    pub dls: f64,
    pub xvs: [f64; 8],
    pub ucvs: [f64; 8],
    pub utvs: f64,
}

pub struct StrOut {
    pub tcc: f64,
    pub xlc: [f64; 8],
    pub vlc: f64,
    pub dlc: f64,
}

pub struct CmpOut {
    pub tcv: f64,
    pub ptv: f64,
    pub xvv: [f64; 8],
}

pub struct FlowsOut {
    pub ftm: [f64; 13],
    pub fcm: [[f64; 13]; 8],
    pub hst: [f64; 13],
    pub xmws: [f64; 13],
    pub cpdh: f64,
    pub uac: f64,
    pub agsp: f64,
}

pub struct HeatOut {
    pub qur: f64,
    pub qus: f64,
    pub quc: f64,
}

// ─── TepDisturbances (Blocks 7–12) ───────────────────────────────────────────
// Atualiza o estado de perturbações e calcula as condições de alimentação.
// Modifica xst[0..2][3], tst[0], tst[3] in-place via referência mutável.

pub struct TepDisturbances;

impl TepDisturbances {
    pub fn compute(
        time: f64,
        idv: &[i32; 20],
        idv_step_mag: &[f64; 20],
        ds: &mut CubicDisturbanceState,
        xst: &mut [[f64; 13]; 8],
        tst: &mut [f64; 13],
    ) -> DistOut {
        let mut idv = *idv;
        for v in idv.iter_mut() {
            *v = if *v > 0 { 1 } else { 0 };
        }

        // Block 8
        ds.channels[0].active = idv[7];
        ds.channels[1].active = idv[7];
        ds.channels[2].active = idv[8];
        ds.channels[3].active = idv[9];
        ds.channels[4].active = idv[10];
        ds.channels[5].active = idv[11];
        ds.channels[6].active = idv[12];
        ds.channels[7].active = idv[12];
        ds.channels[8].active = idv[15];
        ds.channels[9].active = idv[16];
        ds.channels[10].active = idv[17];
        ds.channels[11].active = idv[19];

        // Block 9
        for i in 0..9 {
            if time >= ds.channels[i].t_next {
                let hw = ds.channels[i].t_next - ds.channels[i].t_last;
                let sw = ds.channels[i].a
                    + hw * (ds.channels[i].b + hw * (ds.channels[i].c + hw * ds.channels[i].d));
                let spw =
                    ds.channels[i].b + hw * (2.0 * ds.channels[i].c + 3.0 * hw * ds.channels[i].d);
                ds.channels[i].t_last = ds.channels[i].t_next;
                update_segment(i, sw, spw, ds);
            }
        }

        // Block 10
        for i in 9..12 {
            if time >= ds.channels[i].t_next {
                let hw = ds.channels[i].t_next - ds.channels[i].t_last;
                let sw = ds.channels[i].a
                    + hw * (ds.channels[i].b + hw * (ds.channels[i].c + hw * ds.channels[i].d));
                let spw =
                    ds.channels[i].b + hw * (2.0 * ds.channels[i].c + 3.0 * hw * ds.channels[i].d);
                ds.channels[i].t_last = ds.channels[i].t_next;
                if sw > 0.1 {
                    ds.channels[i].a = sw;
                    ds.channels[i].b = spw;
                    ds.channels[i].c = -(3.0 * sw + 0.2 * spw) / 0.01;
                    ds.channels[i].d = (2.0 * sw + 0.1 * spw) / 0.001;
                    ds.channels[i].t_next = ds.channels[i].t_last + 0.1;
                } else {
                    let hw2 = ds.channels[i].h_span * lcg_rand(-1, ds) + ds.channels[i].h_zero;
                    ds.channels[i].a = 0.0;
                    ds.channels[i].b = 0.0;
                    ds.channels[i].c = ds.channels[i].active as f64 / (hw2 * hw2);
                    ds.channels[i].d = 0.0;
                    ds.channels[i].t_next = ds.channels[i].t_last + hw2;
                }
            }
        }

        // Block 11
        if time == 0.0 {
            for i in 0..12 {
                ds.channels[i].a = ds.channels[i].s_zero;
                ds.channels[i].b = 0.0;
                ds.channels[i].c = 0.0;
                ds.channels[i].d = 0.0;
                ds.channels[i].t_last = 0.0;
                ds.channels[i].t_next = 0.1;
            }
        }

        // Block 12
        xst[0][3] =
            eval_disturbance(0, time, ds) - idv[0] as f64 * 0.03 - idv[1] as f64 * 2.43719e-3;
        xst[1][3] = eval_disturbance(1, time, ds) + idv[1] as f64 * 0.005;
        xst[2][3] = 1.0 - xst[0][3] - xst[1][3];
        tst[0] = eval_disturbance(2, time, ds) + idv[2] as f64 * idv_step_mag[2];
        tst[3] = eval_disturbance(3, time, ds);
        let tcwr = eval_disturbance(4, time, ds) + idv[3] as f64 * idv_step_mag[3];
        let tcws = eval_disturbance(5, time, ds) + idv[4] as f64 * idv_step_mag[4];
        let r1f = eval_disturbance(6, time, ds);
        let r2f = eval_disturbance(7, time, ds);

        DistOut {
            r1f,
            r2f,
            tcwr,
            tcws,
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
        r1f: f64,
        r2f: f64,
        time: f64,
        c: &TepConstants,
    ) -> ReactorOut {
        let mut ucvr = [0.0f64; 8];
        let mut uclr = [0.0f64; 8];
        for i in 0..3 {
            ucvr[i] = slice[i];
        }
        for i in 3..8 {
            uclr[i] = slice[i];
        }
        let etr = slice[8];

        let utlr: f64 = uclr.iter().sum();
        let mut xlr = [0.0f64; 8];
        for i in 0..8 {
            xlr[i] = uclr[i] / utlr;
        }

        let esr = etr / utlr;
        let tcr_init = if time == 0.0 { 120.0 } else { 0.0 };
        let tcr = temperature_from_enthalpy(&xlr, tcr_init, esr, 0, c);
        let tkr = tcr + 273.15;
        let dlr = liquid_density(&xlr, tcr, c);
        let vlr = utlr / dlr;
        let vvr = VTR - vlr;

        let mut ppr = [0.0f64; 8];
        let mut ptr = 0.0f64;
        for i in 0..3 {
            ppr[i] = ucvr[i] * RG * tkr / vvr;
            ptr += ppr[i];
        }
        for i in 3..8 {
            ppr[i] = (c.avp[i] + c.bvp[i] / (tcr + c.cvp[i])).exp() * xlr[i];
            ptr += ppr[i];
        }

        let mut xvr = [0.0f64; 8];
        for i in 0..8 {
            xvr[i] = ppr[i] / ptr;
        }
        let utvr = ptr * vvr / RG / tkr;
        let mut ucvr = ucvr;
        for i in 3..8 {
            ucvr[i] = utvr * xvr[i];
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

        let mut crxr = [0.0f64; 8];
        crxr[0] = -rr[0] - rr[1] - rr[2];
        crxr[2] = -rr[0] - rr[1];
        crxr[3] = -rr[0] - 1.5 * rr[3];
        crxr[4] = -rr[1] - rr[2];
        crxr[5] = rr[2] + rr[3];
        crxr[6] = rr[0];
        crxr[7] = rr[1];
        let rh = rr[0] * HTR[0] + rr[1] * HTR[1];

        ReactorOut {
            tcr,
            tkr,
            ptr,
            xlr,
            vlr,
            dlr,
            vvr,
            xvr,
            ucvr,
            utvr,
            crxr,
            rh,
        }
    }

    pub fn compute_derivatives(f: &FlowsOut, rx: &ReactorOut, qur: f64) -> Vec<f64> {
        let mut yp = vec![0.0f64; 9];
        for i in 0..8 {
            yp[i] = f.fcm[i][6] - f.fcm[i][7] + rx.crxr[i];
        }
        yp[8] = f.hst[6] * f.ftm[6] - f.hst[7] * f.ftm[7] + rx.rh + qur;
        yp
    }
}

// ─── TepSeparator ─────────────────────────────────────────────────────────────
// state_slice[0..3]  = ucvs[0..2], state_slice[3..8] = ucls[3..7], state_slice[8] = ets

pub struct TepSeparator;

impl TepSeparator {
    pub fn compute_thermo(slice: &[f64], tcr: f64, c: &TepConstants) -> SepOut {
        let mut ucvs = [0.0f64; 8];
        let mut ucls = [0.0f64; 8];
        for i in 0..3 {
            ucvs[i] = slice[i];
        }
        for i in 3..8 {
            ucls[i] = slice[i];
        }
        let ets = slice[8];

        let utls: f64 = ucls.iter().sum();
        let mut xls = [0.0f64; 8];
        for i in 0..8 {
            xls[i] = ucls[i] / utls;
        }

        let ess = ets / utls;
        let tcs = temperature_from_enthalpy(&xls, tcr, ess, 0, c);
        let tks = tcs + 273.15;
        let dls = liquid_density(&xls, tcs, c);
        let vls = utls / dls;
        let vvs = VTS - vls;

        let mut pps = [0.0f64; 8];
        let mut pts = 0.0f64;
        for i in 0..3 {
            pps[i] = ucvs[i] * RG * tks / vvs;
            pts += pps[i];
        }
        for i in 3..8 {
            pps[i] = (c.avp[i] + c.bvp[i] / (tcs + c.cvp[i])).exp() * xls[i];
            pts += pps[i];
        }

        let mut xvs = [0.0f64; 8];
        for i in 0..8 {
            xvs[i] = pps[i] / pts;
        }
        let utvs = pts * vvs / RG / tks;
        let mut ucvs = ucvs;
        for i in 3..8 {
            ucvs[i] = utvs * xvs[i];
        }

        SepOut {
            tcs,
            pts,
            xls,
            vls,
            dls,
            xvs,
            ucvs,
            utvs,
        }
    }

    pub fn compute_derivatives(f: &FlowsOut, qus: f64) -> Vec<f64> {
        let mut yp = vec![0.0f64; 9];
        for i in 0..8 {
            yp[i] = f.fcm[i][7] - f.fcm[i][8] - f.fcm[i][9] - f.fcm[i][10];
        }
        yp[8] =
            f.hst[7] * f.ftm[7] - f.hst[8] * f.ftm[8] - f.hst[9] * f.ftm[9] - f.hst[10] * f.ftm[10]
                + qus;
        yp
    }
}

// ─── TepStripper ──────────────────────────────────────────────────────────────
// state_slice[0..8] = uclc[0..7], state_slice[8] = etc

pub struct TepStripper;

impl TepStripper {
    pub fn compute_thermo(slice: &[f64], tcs: f64, c: &TepConstants) -> StrOut {
        let mut uclc = [0.0f64; 8];
        for i in 0..8 {
            uclc[i] = slice[i];
        }
        let etc = slice[8];

        let utlc: f64 = uclc.iter().sum();
        let mut xlc = [0.0f64; 8];
        for i in 0..8 {
            xlc[i] = uclc[i] / utlc;
        }

        let esc = etc / utlc;
        let tcc = temperature_from_enthalpy(&xlc, tcs, esc, 0, c);
        let dlc = liquid_density(&xlc, tcc, c);
        let vlc = utlc / dlc;

        StrOut { tcc, xlc, vlc, dlc }
    }

    pub fn compute_derivatives(f: &FlowsOut, quc: f64) -> Vec<f64> {
        let mut yp = vec![0.0f64; 9];
        for i in 0..8 {
            yp[i] = f.fcm[i][11] - f.fcm[i][12];
        }
        yp[8] = f.hst[3] * f.ftm[3] + f.hst[10] * f.ftm[10]
            - f.hst[4] * f.ftm[4]
            - f.hst[12] * f.ftm[12]
            + quc;
        yp
    }
}

// ─── TepCompressor ────────────────────────────────────────────────────────────
// state_slice[0..8] = ucvv[0..7], state_slice[8] = etv

pub struct TepCompressor;

impl TepCompressor {
    pub fn compute_thermo(slice: &[f64], tcs: f64, c: &TepConstants) -> CmpOut {
        let mut ucvv = [0.0f64; 8];
        for i in 0..8 {
            ucvv[i] = slice[i];
        }
        let etv = slice[8];

        let utvv: f64 = ucvv.iter().sum();
        let mut xvv = [0.0f64; 8];
        for i in 0..8 {
            xvv[i] = ucvv[i] / utvv;
        }

        let esv = etv / utvv;
        let tcv = temperature_from_enthalpy(&xvv, tcs, esv, 2, c);
        let tkv = tcv + 273.15;
        let ptv = utvv * RG * tkv / VTV;

        CmpOut { tcv, ptv, xvv }
    }

    pub fn compute_derivatives(f: &FlowsOut) -> Vec<f64> {
        let mut yp = vec![0.0f64; 9];
        for i in 0..8 {
            yp[i] =
                f.fcm[i][0] + f.fcm[i][1] + f.fcm[i][2] + f.fcm[i][4] + f.fcm[i][8] - f.fcm[i][5];
        }
        yp[8] = f.hst[0] * f.ftm[0]
            + f.hst[1] * f.ftm[1]
            + f.hst[2] * f.ftm[2]
            + f.hst[4] * f.ftm[4]
            + f.hst[8] * f.ftm[8]
            - f.hst[5] * f.ftm[5];
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
        rx: &ReactorOut,
        sep: &SepOut,
        str: &StrOut,
        cmp: &CmpOut,
        xst: &mut [[f64; 13]; 8],
        tst: &mut [f64; 13],
        sfr: &mut [f64; 8],
        vpos: &[f64; 12],
        idv: &[i32; 20],
        c: &TepConstants,
        time: f64,
        ds: &CubicDisturbanceState,
    ) -> FlowsOut {
        // Block 19: composições de corrente a partir dos estados de unidade
        for i in 0..8 {
            xst[i][5] = cmp.xvv[i];
            xst[i][7] = rx.xvr[i];
            xst[i][8] = sep.xvs[i];
            xst[i][9] = sep.xvs[i];
            xst[i][10] = sep.xls[i];
            xst[i][12] = str.xlc[i];
        }

        // Block 20: pesos moleculares das correntes
        let mut xmws = [0.0f64; 13];
        for i in 0..8 {
            xmws[0] += xst[i][0] * c.xmw[i];
            xmws[1] += xst[i][1] * c.xmw[i];
            xmws[5] += xst[i][5] * c.xmw[i];
            xmws[7] += xst[i][7] * c.xmw[i];
            xmws[8] += xst[i][8] * c.xmw[i];
            xmws[9] += xst[i][9] * c.xmw[i];
        }

        // Block 21: temperaturas e entalpias das correntes
        tst[5] = cmp.tcv;
        tst[7] = rx.tcr;
        tst[8] = sep.tcs;
        tst[9] = sep.tcs;
        tst[10] = sep.tcs;
        tst[12] = str.tcc;
        let mut hst = [0.0f64; 13];
        hst[0] = mixture_enthalpy(&xst_col(xst, 0), tst[0], 1, c);
        hst[1] = mixture_enthalpy(&xst_col(xst, 1), tst[1], 1, c);
        hst[2] = mixture_enthalpy(&xst_col(xst, 2), tst[2], 1, c);
        hst[3] = mixture_enthalpy(&xst_col(xst, 3), tst[3], 1, c);
        hst[5] = mixture_enthalpy(&xst_col(xst, 5), tst[5], 1, c);
        hst[7] = mixture_enthalpy(&xst_col(xst, 7), tst[7], 1, c);
        hst[8] = mixture_enthalpy(&xst_col(xst, 8), tst[8], 1, c);
        hst[9] = hst[8];
        hst[10] = mixture_enthalpy(&xst_col(xst, 10), tst[10], 0, c);
        hst[12] = mixture_enthalpy(&xst_col(xst, 12), tst[12], 0, c);

        // Blocks 22–24: fluxos mássicos
        let mut ftm = [0.0f64; 13];
        ftm[0] = vpos[0] * VRNG[0] / 100.0;
        ftm[1] = vpos[1] * VRNG[1] / 100.0;
        ftm[2] = vpos[2] * (1.0 - idv[5] as f64) * VRNG[2] / 100.0;
        ftm[3] = vpos[3] * (1.0 - idv[6] as f64 * 0.2) * VRNG[3] / 100.0 + 1e-10;
        ftm[10] = vpos[6] * VRNG[6] / 100.0;
        ftm[12] = vpos[7] * VRNG[7] / 100.0;
        let uac = vpos[8] * VRNG[8] * (1.0 + eval_disturbance(8, time, ds)) / 100.0;
        let agsp = (vpos[11] + 150.0) / 100.0;

        ftm[5] = 1937.6 * (cmp.ptv - rx.ptr).max(0.0).sqrt() / xmws[5];
        ftm[7] = 4574.21
            * (rx.ptr - sep.pts).max(0.0).sqrt()
            * (1.0 - 0.25 * eval_disturbance(11, time, ds))
            / xmws[7];
        ftm[9] = vpos[5] * 0.151169 * (sep.pts - 760.0).max(0.0).sqrt() / xmws[9];

        let pr = (cmp.ptv / sep.pts).max(1.0).min(CPPRMX);
        let flcoef = CPFLMX / 1.197;
        let mut flms = CPFLMX + flcoef * (1.0 - pr.powi(3));
        let cpdh =
            flms * (sep.tcs + 273.15) * 1.8e-6 * 1.9872 * (cmp.ptv - sep.pts) / (xmws[8] * sep.pts);
        flms -= vpos[4] * 53.349 * (cmp.ptv - sep.pts).max(0.0).sqrt();
        flms = flms.max(1e-3);
        ftm[8] = flms / xmws[8];
        hst[8] += cpdh / ftm[8];

        // Block 25: fluxos por componente
        let mut fcm = [[0.0f64; 13]; 8];
        for i in 0..8 {
            fcm[i][0] = xst[i][0] * ftm[0];
            fcm[i][1] = xst[i][1] * ftm[1];
            fcm[i][2] = xst[i][2] * ftm[2];
            fcm[i][3] = xst[i][3] * ftm[3];
            fcm[i][5] = xst[i][5] * ftm[5];
            fcm[i][7] = xst[i][7] * ftm[7];
            fcm[i][8] = xst[i][8] * ftm[8];
            fcm[i][9] = xst[i][9] * ftm[9];
            fcm[i][10] = xst[i][10] * ftm[10];
            fcm[i][12] = xst[i][12] * ftm[12];
        }

        // Blocks 26–31: fracionamento no stripper
        if ftm[10] > 0.1 {
            let tmpfac = if str.tcc > 170.0 {
                str.tcc - 120.262
            } else if str.tcc < 5.292 {
                0.1
            } else {
                363.744 / (177.0 - str.tcc) - 2.22579488
            };
            let vovrl = ftm[3] / ftm[10] * tmpfac;
            sfr[3] = 8.5010 * vovrl / (1.0 + 8.5010 * vovrl);
            sfr[4] = 11.402 * vovrl / (1.0 + 11.402 * vovrl);
            sfr[5] = 11.795 * vovrl / (1.0 + 11.795 * vovrl);
            sfr[6] = 0.0480 * vovrl / (1.0 + 0.0480 * vovrl);
            sfr[7] = 0.0242 * vovrl / (1.0 + 0.0242 * vovrl);
        } else {
            sfr[3] = 0.9999;
            sfr[4] = 0.999;
            sfr[5] = 0.999;
            sfr[6] = 0.99;
            sfr[7] = 0.98;
        }

        let mut fin = [0.0f64; 8];
        for i in 0..8 {
            fin[i] = fcm[i][3] + fcm[i][10];
        }
        ftm[4] = 0.0;
        ftm[11] = 0.0;
        for i in 0..8 {
            fcm[i][4] = sfr[i] * fin[i];
            fcm[i][11] = fin[i] - fcm[i][4];
            ftm[4] += fcm[i][4];
            ftm[11] += fcm[i][11];
        }
        for i in 0..8 {
            xst[i][4] = fcm[i][4] / ftm[4];
            xst[i][11] = fcm[i][11] / ftm[11];
        }
        tst[4] = str.tcc;
        tst[11] = str.tcc;
        hst[4] = mixture_enthalpy(&xst_col(xst, 4), tst[4], 1, c);
        hst[11] = mixture_enthalpy(&xst_col(xst, 11), tst[11], 0, c);
        ftm[6] = ftm[5];
        hst[6] = hst[5];
        tst[6] = tst[5];
        for i in 0..8 {
            xst[i][6] = xst[i][5];
            fcm[i][6] = fcm[i][5];
        }

        FlowsOut {
            ftm,
            fcm,
            hst,
            xmws,
            cpdh,
            uac,
            agsp,
        }
    }
}

// ─── TepHeat (Blocks 32–34) ───────────────────────────────────────────────────
// Calcula transferência de calor no reator, separador e stripper.

pub struct TepHeat;

impl TepHeat {
    pub fn compute(
        rx: &ReactorOut,
        str: &StrOut,
        f: &FlowsOut,
        twr: f64, // state[36] — temperatura de retorno da água de resfriamento do reator
        tws: f64, // state[37] — temperatura de retorno do separador
        tcr: f64, // = rx.tcr (temperatura do efluente do reator, = tst[7] do flows)
        time: f64,
        ds: &CubicDisturbanceState,
    ) -> HeatOut {
        // Block 32: reator
        let uarlev = if rx.vlr / 7.8 > 50.0 {
            1.0
        } else if rx.vlr / 7.8 < 10.0 {
            0.0
        } else {
            0.025 * rx.vlr / 7.8 - 0.25
        };
        let uar = uarlev * (-0.5 * f.agsp * f.agsp + 2.75 * f.agsp - 2.5) * 855490e-6;
        let qur = uar * (twr - rx.tcr) * (1.0 - 0.35 * eval_disturbance(9, time, ds));

        // Block 33: separador
        let uas = 0.404655 * (1.0 - 1.0 / (1.0 + (f.ftm[7] / 3528.73).powi(4)));
        let qus = uas * (tws - tcr) * (1.0 - 0.25 * eval_disturbance(10, time, ds));

        // Block 34: stripper (condensador)
        let quc = if str.tcc < 100.0 {
            f.uac * (100.0 - str.tcc)
        } else {
            0.0
        };

        HeatOut { qur, qus, quc }
    }
}

// ─── TepMeasurements (Block 35) ──────────────────────────────────────────────
// Snapshot físico — sem ruído. Preenche xmeas[0..22].

pub struct TepMeasurements;

impl TepMeasurements {
    pub fn compute(
        rx: &ReactorOut,
        sep: &SepOut,
        str: &StrOut,
        cmp: &CmpOut,
        f: &FlowsOut,
        h: &HeatOut,
        twr: f64,
        tws: f64,
        xmeas: &mut [f64; 41],
    ) -> bool {
        xmeas[0] = f.ftm[2] * 0.359 / 35.3145;
        xmeas[1] = f.ftm[0] * f.xmws[0] * 0.454;
        xmeas[2] = f.ftm[1] * f.xmws[1] * 0.454;
        xmeas[3] = f.ftm[3] * 0.359 / 35.3145;
        xmeas[4] = f.ftm[8] * 0.359 / 35.3145;
        xmeas[5] = f.ftm[5] * 0.359 / 35.3145;
        xmeas[6] = (rx.ptr - 760.0) / 760.0 * 101.325;
        xmeas[7] = (rx.vlr - 84.6) / 666.7 * 100.0;
        xmeas[8] = rx.tcr;
        xmeas[9] = f.ftm[9] * 0.359 / 35.3145;
        xmeas[10] = sep.tcs;
        xmeas[11] = (sep.vls - 27.5) / 290.0 * 100.0;
        xmeas[12] = (sep.pts - 760.0) / 760.0 * 101.325;
        xmeas[13] = f.ftm[10] / sep.dls / 35.3145;
        xmeas[14] = (str.vlc - 78.25) / VTC * 100.0;
        xmeas[15] = (cmp.ptv - 760.0) / 760.0 * 101.325;
        xmeas[16] = f.ftm[12] / str.dlc / 35.3145;
        xmeas[17] = str.tcc;
        xmeas[18] = h.quc * 1.04e3 * 0.454;
        xmeas[19] = f.cpdh * 0.29307e3;
        xmeas[20] = twr;
        xmeas[21] = tws;

        // Block 36: detecção de shutdown
        xmeas[6] > 3000.0
            || rx.vlr / 35.3145 > 24.0
            || rx.vlr / 35.3145 < 2.0
            || xmeas[8] > 175.0
            || sep.vls / 35.3145 > 12.0
            || sep.vls / 35.3145 < 1.0
            || str.vlc / 35.3145 > 8.0
            || str.vlc / 35.3145 < 1.0
    }
}
