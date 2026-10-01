/* tep/disturbance/idv3.rs */

/** IDV(3) — Step: temperatura do D feed (Table 8, Downs & Vogel 1993; `teprob.f`:
`TST(1) += IDV(3)*5.0`). Temperatura de alimentação de D sobe em step — D é alimentado como
líquido, temperatura mais alta muda o enthalpy de entrada e a taxa de vaporização dentro do
reator. Efeito moderado sobre temperatura e pressão do reator.

Intercepta `flows.d_feed_temperature` — a única das quatro temperaturas de feed (D/E/A/A&C) que
`units::feed::Feed` publica como chave própria (ver `feed.rs::d_feed_temperature`); é a única que
`teprob.f` deixa perturbável, E/A continuam sendo a constante `FEED_TEMPERATURE` direta, sem chave.
*/
#[monjolo::disturbance(key = "disturbance.idv3", intercepts = "flows.d_feed_temperature")]
pub struct Idv3;

impl Idv3 {
    fn disturb(&self) -> f64 {
        self.raw() + 5.0
    }
}
