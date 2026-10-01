/* tep/disturbance/idv5.rs */

/* IDV(5) — Step: temperatura de entrada da água de resfriamento do condensador, +5°C (Table 8,
Downs & Vogel 1993). Mesmo mecanismo do IDV(4), mas no condensador do separador. Reduz a condensação
no separador, aumenta a fração de vapor no reciclo e eleva a carga sobre o compressor.

CASCA — ainda não implementado. Ponto de interceptação: `units/separator.rs`. Mesma lacuna
estrutural do IDV(4), lado separador — ver `idv4.rs`. Ver `docs/05-disturbios.md`.
*/
