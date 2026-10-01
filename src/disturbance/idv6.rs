/* tep/disturbance/idv6.rs */

/** IDV(6) — Step: perda total do A feed (Table 8, Downs & Vogel 1993; `teprob.f`:
`FTM(3) = VPOS(3)*(1-IDV(6))*VRNG(3)/100` — o multiplicador `(1-IDV(6))` aplica DIRETO sobre a
vazão já escalada pela válvula, não sobre a posição). Válvula do A feed fecha completamente
(fator vira 0 quando ligado). Sem A no reator, a reação para gradualmente — o inventário líquido
cai (sem produto G/H sendo gerado) e a composição do loop muda drasticamente. Distúrbio severo —
a planta sem controle adequado atinge ISD por nível baixo.

Intercepta `flows.stream_flow.1` — exatamente o que `units::feed::Feed::a_feed_flow` publica.
*/
#[monjolo::disturbance(key = "disturbance.idv6", intercepts = "flows.stream_flow.1")]
pub struct Idv6;

impl Idv6 {
    fn disturb(&self) -> f64 {
        0.0
    }
}
