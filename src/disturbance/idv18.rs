/* tep/disturbance/idv18.rs */

/* IDV(18) — Aleatório (canal de pulso): remoção de calor do separador/condensador, QUS.
Listado como "Unknown" no paper, mas `teprob.f` dá fórmula concreta: `QUS = UAS*(TWS-TST(8))*(1 -
0.25*TESUB8(11,TIME))` — mesma mecânica do IDV(17) (Block 10, canal 10), aplicada à troca térmica
do separador em vez do reator.

CASCA — ainda não implementado. Ponto de interceptação: `units/separator.rs` (linha 68, mesmo
padrão de placeholder do IDV(17)). Mesma lacuna de mecanismo — precisa de canal de pulso com `time`.
Ver `docs/05-disturbios.md`.
*/
