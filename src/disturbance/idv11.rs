/* tep/disturbance/idv11.rs */

/* IDV(11) — Aleatório: variação na temperatura de entrada da água de resfriamento do reator
(Table 8, Downs & Vogel 1993). Flutuação contínua na temperatura de entrada do CW do reator. Torna
o controle de temperatura do reator inerentemente mais difícil — o controlador de CW precisa
compensar uma perturbação de entrada variável.

CASCA — ainda não implementado, E sem efeito possível hoje mesmo se implementado: mesma variável
(`tcwr`) e mesma lacuna do IDV(4) — `twr` está congelada desde o Exp 24, sem depender de `tcwr`
nenhum (ver `idv4.rs`). Soma-se a lacuna de mecanismo do IDV(8)/(9)/(10) — precisa de canal cúbico
com `time`. Ver `docs/05-disturbios.md`.
*/
