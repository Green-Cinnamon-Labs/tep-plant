/* Documentação: docs/16-feed.md */

use crate::physics::constants::{TepConstants, TEP_SPECIES};
use monjolo::chemistry::Mixture;

const FEED_D_RANGE: f64 = 400.0;
const FEED_E_RANGE: f64 = 400.0;
const FEED_A_RANGE: f64 = 100.0;
const FEED_AC_RANGE: f64 = 1500.0;

pub(crate) const FEED_D_COMPOSITION: [f64; 8] = [0.0, 0.0001, 0.0, 0.9999, 0.0, 0.0, 0.0, 0.0];
pub(crate) const FEED_E_COMPOSITION: [f64; 8] = [0.0, 0.0, 0.0, 0.0, 0.9999, 0.0001, 0.0, 0.0];
pub(crate) const FEED_A_COMPOSITION: [f64; 8] = [0.9999, 0.0001, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
pub(crate) const FEED_AC_COMPOSITION: [f64; 8] = [0.4850, 0.0050, 0.5100, 0.0, 0.0, 0.0, 0.0, 0.0];
pub(crate) const FEED_TEMPERATURE: f64 = 45.0;

#[monjolo::dynamic_model(tasks)]
pub struct Feed {
    constants: TepConstants,
}

#[monjolo::tasks(species = TEP_SPECIES, len = 8)]
impl Feed {
    #[task]
    fn d_feed_flow(&self) {
        offer::flows__stream_flow__2 = need::valve__feed_d__position * FEED_D_RANGE / 100.0;
    }

    #[task]
    fn e_feed_flow(&self) {
        offer::flows__stream_flow__3 = need::valve__feed_e__position * FEED_E_RANGE / 100.0;
    }

    #[task]
    fn a_feed_flow(&self) {
        offer::flows__stream_flow__1 = need::valve__feed_a__position * FEED_A_RANGE / 100.0;
    }

    #[task]
    fn ac_feed_flow(&self) {
        offer::flows__stream_flow__4 = need::valve__feed_ac__position * FEED_AC_RANGE / 100.0 + 1e-10;
    }

    #[task]
    fn ac_feed_composition(&self) {
        offer::flows__stream4_composition::<Mixture> = Mixture::new(FEED_AC_COMPOSITION, &TEP_SPECIES);
    }

    #[task]
    fn d_feed_mol_weight(&self) {
        offer::flows__d_feed_mol_weight = Mixture::new(FEED_D_COMPOSITION, &TEP_SPECIES).dot(&self.constants.xmw);
    }

    #[task]
    fn e_feed_mol_weight(&self) {
        offer::flows__e_feed_mol_weight = Mixture::new(FEED_E_COMPOSITION, &TEP_SPECIES).dot(&self.constants.xmw);
    }

    #[task]
    fn xmeas_readings(&self) {
        offer::xmeas__stream1__flow_rate = need::flows__stream_flow__1 * 0.359 / 35.3145;
        offer::xmeas__stream2__flow_rate = need::flows__stream_flow__2 * need::flows__d_feed_mol_weight * 0.454;
        offer::xmeas__stream3__flow_rate = need::flows__stream_flow__3 * need::flows__e_feed_mol_weight * 0.454;
        offer::xmeas__stream4__flow_rate = need::flows__stream_flow__4 * 0.359 / 35.3145;
    }
}
