/* tep/disturbance/idv8.rs */

/* IDV(8) — Aleatório: variação na composição A/B/C do feed combinado (stream 4) (Table 8, Downs &
Vogel 1993). Ruído randômico contínuo na composição da alimentação combinada. Mais difícil de
rejeitar do que distúrbios em step porque não tem ponto de operação estacionário alternativo —
exige malhas de controle robustas a variação persistente.

CASCA — ainda não implementado. Mesmo ponto de IDV(1)/(2) (`flows.stream4_composition`), perfil
aleatório em vez de step. **Não cabe no `#[monjolo::disturbance]` atual** — `disturb(&self)` é uma
função PURA, sem estado que avança por tick; "aleatório" precisa de `time` e de um canal cúbico
contínuo (mecânica que existiu em `Disturbance`/`state.rs`, código morto apagado nesta issue — ver
`mod.rs`), ainda não reconstruída sob o padrão novo. Ver `docs/05-disturbios.md`.

TODO: desenhar e implementar um mecanismo de canal cúbico (com `time`, estado que avança por
tick) compatível com `#[monjolo::disturbance]`, antes de implementar este IDV.
*/
