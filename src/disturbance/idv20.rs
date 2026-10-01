/* tep/disturbance/idv20.rs */

/* IDV(20) — Aleatório (canal de pulso): coeficiente de vazão reator→separador.
Listado como "Unknown" no paper, mas `teprob.f` (Block 23) dá fórmula concreta: multiplicador
`(1 - 0.25*TESUB8(12,TIME))` sobre o cálculo de vazão de vapor reator→separador. Canal 11
(0-indexado), mecânica de pulso (Block 10, igual IDV(17)/(18)).

CASCA — ainda não implementado, mas **já tem um placeholder no código**: `units/reactor.rs`,
`flow_to_separator()`: `... * (1.0 - 0.25 * 0.0) / mol_weight` — o `0.0` é onde este canal entra.
Mesma lacuna de mecanismo dos demais "aleatórios" — precisa de canal de pulso com `time`. Ver
`docs/05-disturbios.md`.

TODO: mesmos dois bloqueios do IDV(17)/(18) (canal de pulso pendente + literal `0.0` precisa
virar chave publicada), lado vazão reator→separador.
*/
