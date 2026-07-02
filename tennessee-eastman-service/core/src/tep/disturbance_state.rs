// dynamics/tep/disturbance_state.rs

use crate::disturbance::cubic::{CubicDisturbanceState, DisturbanceChannel};

/** ## TEP-specific disturbance state.

 Wraps `CubicDisturbanceState` and initializes the 12 disturbance
 channels with the shape parameters hardcoded in TEINIT from teprob.f.

 Each channel maps to a specific process variable:
   0  → XST(1,4) — A/C feed ratio composition (IDV 8)
   1  → XST(2,4) — B composition in feed 4    (IDV 8)
   2  → TST(1)   — D feed temperature          (IDV 9)
   3  → TST(4)   — C feed temperature          (IDV 10)
   4  → TCWR     — reactor cooling water temp  (IDV 11)
   5  → TCWS     — condenser cooling water temp(IDV 12)
   6  → R1F      — reaction kinetics factor 1  (IDV 13)
   7  → R2F      — reaction kinetics factor 2  (IDV 13)
   8  → UAC      — condenser heat transfer     (IDV 16)
   9  → reactor cooling disturbance            (IDV 17)
   10 → separator cooling disturbance          (IDV 18)
   11 → feed flow disturbance                  (IDV 20)
*/
pub struct TepDisturbanceState {
    pub inner: CubicDisturbanceState,
}

impl TepDisturbanceState {
    /// Initialize all 12 TEP disturbance channels from TEINIT values.
    pub fn new() -> Self {
        // Parameters from TEINIT in teprob.f
        // (h_span, h_zero, s_span, s_zero, sp_span)
        let channel_params: [(f64, f64, f64, f64, f64); 12] = [
            (0.2,  0.5,  0.03,  0.485, 0.0), // channel 0
            (0.7,  1.0,  0.003, 0.005, 0.0), // channel 1
            (0.25, 0.5,  10.0,  45.0,  0.0), // channel 2
            (0.7,  1.0,  10.0,  45.0,  0.0), // channel 3
            (0.15, 0.25, 10.0,  35.0,  0.0), // channel 4
            (0.15, 0.25, 10.0,  40.0,  0.0), // channel 5
            (1.0,  2.0,  0.25,  1.0,   0.0), // channel 6
            (1.0,  2.0,  0.25,  1.0,   0.0), // channel 7
            (0.4,  0.5,  0.25,  0.0,   0.0), // channel 8
            (1.5,  2.0,  0.0,   0.0,   0.0), // channel 9
            (2.0,  3.0,  0.0,   0.0,   0.0), // channel 10
            (1.5,  2.0,  0.0,   0.0,   0.0), // channel 11
        ];

        let channels = channel_params
            .iter()
            .map(|&(h_span, h_zero, s_span, s_zero, sp_span)| {
                DisturbanceChannel::new(h_span, h_zero, s_span, s_zero, sp_span)
            })
            .collect();

        Self {
            inner: CubicDisturbanceState::new(channels, 4_651_207_995.0),
        }
    }
}

impl Default for TepDisturbanceState {
    fn default() -> Self {
        Self::new()
    }
}