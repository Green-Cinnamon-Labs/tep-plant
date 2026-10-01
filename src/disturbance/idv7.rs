/* tep/disturbance/idv7.rs */

/** IDV(7) — Step: queda de pressão no header de C (Table 8, Downs & Vogel 1993; `teprob.f`:
`FTM(4) = VPOS(4)*(1-IDV(7)*0.2)*VRNG(4)/100 + 1e-10` — o multiplicador `(1-0.2·IDV(7))` aplica
sobre a vazão já escalada pela válvula, ANTES do `+1e-10` final). A pressão de fornecimento de C
cai, reduzindo o fluxo de C pro reator. Efeito similar ao IDV(6) pra C: menos reagente disponível,
reação desbalanceada. Menos severo que o IDV(6) porque a redução é parcial (20%, não total) e
afeta a vazão combinada A&C inteira, não só C isoladamente — mesma limitação de sempre.

Intercepta `flows.stream_flow.4` — exatamente o que `units::feed::Feed::ac_feed_flow` publica
(já inclui o `+1e-10`; `disturb()` subtrai, aplica os 20%, soma de volta, pra bater byte a byte
com a ordem de operações do `teprob.f`).
*/
#[monjolo::disturbance(key = "disturbance.idv7", intercepts = "flows.stream_flow.4")]
pub struct Idv7;

impl Idv7 {
    fn disturb(&self) -> f64 {
        (self.raw() - 1e-10) * 0.8 + 1e-10
    }
}
