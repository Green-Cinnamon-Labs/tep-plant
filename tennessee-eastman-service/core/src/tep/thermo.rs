use crate::tep::constants::TepConstants;

/** ## Compute mixture enthalpy.
Given molar fractions `z[8]` and temperature `t` (°C), returns the total enthalpy of the mixture.
`ity` controls the phase model:
  0 = liquid
  1 = vapor
  2 = vapor with ideal gas correction (PV)
Direct translation of SUBROUTINE TESUB1 from teprob.f
*/
pub fn mixture_enthalpy(z: &[f64; 8], t: f64, ity: i32, constants: &TepConstants) -> f64 {
    let mut h = 0.0_f64;

    if ity == 0 {
        // Liquid: HI = T*(AH + BH*T/2 + CH*T²/3) * 1.8
        for i in 0..8 {
            let hi = t * (constants.ah[i] + constants.bh[i] * t / 2.0 + constants.ch[i] * t * t / 3.0);
            h += z[i] * constants.xmw[i] * 1.8 * hi;
        }
    } else {
        // Vapor: HI = T*(AG + BG*T/2 + CG*T²/3) * 1.8 + AV
        for i in 0..8 {
            let hi = t * (constants.ag[i] + constants.bg[i] * t / 2.0 + constants.cg[i] * t * t / 3.0);
            h += z[i] * constants.xmw[i] * (1.8 * hi + constants.av[i]);
        }
    }

    // Ideal gas correction (ity == 2): H -= R*(T + 273.15)
    if ity == 2 {
        h -= 3.57696e-6 * (t + 273.15);
    }

    h
}

/** ## Compute temperature from enthalpy via Newton-Raphson.
Solves T such that `mixture_enthalpy(z, T, ity) == h_target`.
Starts from `t_init` and iterates until |ΔT| < 1e-12 or 100 iterations are exhausted (returns `t_init` on failure).
Direct translation of SUBROUTINE TESUB2 from teprob.f
*/
pub fn temperature_from_enthalpy(
    z: &[f64; 8],
    t_init: f64,
    h_target: f64,
    ity: i32,
    constants: &TepConstants,
) -> f64 {
    let mut t = t_init;

    for _ in 0..100 {
        let h_test = mixture_enthalpy(z, t, ity, constants);
        let dh = enthalpy_derivative(z, t, ity, constants);
        let dt = -(h_test - h_target) / dh;
        t += dt;
        if dt.abs() < 1.0e-12 {
            return t;
        }
    }

    t_init
}

/** ## Compute dH/dT (enthalpy derivative with respect to temperature).
Used as the Jacobian in the Newton-Raphson loop of
`temperature_from_enthalpy`.
Direct translation of SUBROUTINE TESUB3 from teprob.f
*/
pub fn enthalpy_derivative(z: &[f64; 8], t: f64, ity: i32, constants: &TepConstants) -> f64 {
    let mut dh = 0.0_f64;

    if ity == 0 {
        for i in 0..8 {
            let dhi = (constants.ah[i] + constants.bh[i] * t + constants.ch[i] * t * t) * 1.8;
            dh += z[i] * constants.xmw[i] * dhi;
        }
    } else {
        for i in 0..8 {
            let dhi = (constants.ag[i] + constants.bg[i] * t + constants.cg[i] * t * t) * 1.8;
            dh += z[i] * constants.xmw[i] * dhi;
        }
    }

    if ity == 2 {
        dh -= 3.57696e-6;
    }

    dh
}

/** ## Compute liquid mixture density.

 Empirical correlation:
   V = Σ( x_i * XMW_i / (AD_i + (BD_i + CD_i*T)*T) )
   ρ = 1 / V

 Direct translation of SUBROUTINE TESUB4 from teprob.f
*/
pub fn liquid_density(x: &[f64; 8], t: f64, constants: &TepConstants) -> f64 {
    let v: f64 = (0..8)
        .map(|i| x[i] * constants.xmw[i] / (constants.ad[i] + (constants.bd[i] + constants.cd[i] * t) * t))
        .sum();
    1.0 / v
}
