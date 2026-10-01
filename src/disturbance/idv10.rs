/* tep/disturbance/idv10.rs */

/* IDV(10) — Aleatório: variação na temperatura do C feed (stream 4) (Table 8, Downs & Vogel 1993).
Ruído randômico na temperatura de C na alimentação combinada A&C. Análogo ao IDV(9) mas para o
outro reagente gasoso principal.

CASCA — ainda não implementado. Ponto de interceptação: `units/feed.rs` (`FEED_TEMPERATURE`),
consumida em `units/stripper.rs` — mesma constante compartilhada do IDV(3)/(9), lado A&C. Mesma
lacuna de mecanismo do IDV(8)/(9) — precisa de canal cúbico com `time`. Ver `docs/05-disturbios.md`.

TODO: dois bloqueios — (1) canal cúbico pendente (IDV(8)); (2) `FEED_TEMPERATURE` do lado A&C
ainda não foi separada numa chave própria (só D foi, pro IDV(3)/(9) — ver `units/feed.rs`).
*/
