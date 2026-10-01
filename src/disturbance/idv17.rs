/* tep/disturbance/idv17.rs */

/* IDV(17) — Aleatório (canal de pulso): remoção de calor do reator, QUR.
Listado como "Unknown" no paper, mas `teprob.f` dá fórmula concreta: `QUR = UAR*(TWR-TCR)*(1 -
0.35*TESUB8(10,TIME))` — perturbação multiplicativa sobre a taxa de remoção de calor do reator.
Canal 9 (0-indexado), mecânica de pulso/duração aleatória (Block 10, diferente do canal cúbico
contínuo do IDV(16)).

CASCA — ainda não implementado, mas **já tem um placeholder no código**: `units/reactor.rs`,
`heat_exchange()`: `uar * (twr - reactor_temperature) * (1.0 - 0.35 * 0.0)` — o `0.0` é onde este
canal entra. Mesma lacuna de mecanismo dos demais "aleatórios" — precisa de canal de pulso com
`time`. Ver `docs/05-disturbios.md`.

TODO: dois bloqueios — (1) canal de pulso pendente (variante do canal do IDV(8)); (2) o `0.0` em
`reactor.rs` é um literal, precisa virar uma chave publicada antes de ter o que interceptar.
*/
