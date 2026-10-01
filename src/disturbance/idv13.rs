/* tep/disturbance/idv13.rs */

/* IDV(13) — Deriva lenta: cinética de reação (Table 8, Downs & Vogel 1993).
A constante de velocidade da reação deriva lentamente ao longo do tempo (horas). Simula
envelhecimento de catalisador ou mudança de condições de processo. Difícil de detectar sem
instrumentação adequada; a planta opera aparentemente normal e só falha em horizonte longo.

CASCA — ainda não implementado. Ponto de interceptação: `units/reactor.rs`
(`REACTION_FACTOR_1/2_NOMINAL`, usadas dentro de `REACTIONS`). **Não cabe no mecanismo de
interceptação atual nem no canal cúbico dos demais "aleatórios"** — "deriva lenta" precisa de
estado que ACUMULA ao longo de horas (não um perfil de ruído de curto prazo), mecanismo ainda a
definir. Ver `docs/05-disturbios.md`.

TODO: desenhar um mecanismo de deriva ACUMULATIVA (estado que avança com `time`, sem reset, horas
de horizonte) — diferente do canal cúbico dos demais "aleatórios", ainda a definir.
*/
