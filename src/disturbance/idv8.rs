/* tep/disturbance/idv8.rs */

/* IDV(8) — Aleatório: variação na composição A/B/C do feed combinado (stream 4) (Table 8, Downs &
Vogel 1993). Ruído randômico contínuo na composição da alimentação combinada. Mais difícil de
rejeitar do que distúrbios em step porque não tem ponto de operação estacionário alternativo —
exige malhas de controle robustas a variação persistente.

CASCA — ainda não implementado. Mesmo ponto de IDV(1)/(2) (`flows.stream4_composition`), perfil
aleatório em vez de step. **Não cabe no mecanismo de interceptação atual** (`#[disturbance(...)]`
sozinho é uma função PURA de `(active, valor cru)` — "aleatório" precisa de `time` e de estado que
avança por tick, a mesma mecânica de canal cúbico contínuo que `state.rs`/`Disturbance` (código
morto, ver `mod.rs`) já tem, ainda não portada pro padrão novo). Ver `docs/05-disturbios.md`.
*/
