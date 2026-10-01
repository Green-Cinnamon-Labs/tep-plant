/* tep/disturbance/idv16.rs */

/* IDV(16) — Aleatório (canal cúbico contínuo): coeficiente de troca térmica do condensador, UAC.
Listado como "Unknown" no paper, mas `teprob.f` dá fórmula concreta: `UAC =
VPOS(9)*VRNG(9)*(1 + TESUB8(9,TIME))/100` — perturbação multiplicativa sobre o coeficiente de troca
térmica do condensador, derivado da posição de válvula. Canal 8 (0-indexado) de
`TepDisturbanceState`, mesma mecânica de canal cúbico contínuo (Block 9) usada por IDV(1)-(3)/(8)-(10).

CASCA — ainda não implementado. Sem consumidor no port Rust ainda (`UAC`/condensador não publica
esse coeficiente hoje — ponto mais próximo: `units/stripper.rs`, `condenser_ua`). Mesma lacuna de
mecanismo dos demais "aleatórios" — precisa de canal cúbico com `time`. Ver `docs/05-disturbios.md`.
*/
