/* tep/disturbance/idv12.rs */

/* IDV(12) — Aleatório: variação na temperatura de entrada da água de resfriamento do condensador
(Table 8, Downs & Vogel 1993). Análogo ao IDV(11) para o condensador. Afeta a eficiência de
separação e a temperatura do separador (XMEAS(11)).

CASCA — ainda não implementado. Ponto de interceptação: `units/separator.rs` — mesmo ponto do
IDV(5). Mesma lacuna de mecanismo do IDV(8)-(11) — precisa de canal cúbico com `time`. Ver
`docs/05-disturbios.md`.
*/
