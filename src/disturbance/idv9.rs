/* tep/disturbance/idv9.rs */

/* IDV(9) — Aleatório: variação na temperatura do D feed (stream 2) (Table 8, Downs & Vogel 1993).
Ruído randômico na temperatura de entrada de D. Efeito entálpico contínuo sobre o reator.
Geralmente mais brando que os distúrbios de composição porque a entalpia de D é uma perturbação de
segunda ordem.

CASCA — ainda não implementado. Mesmo ponto e mesma ressalva do IDV(3) (`FEED_TEMPERATURE`
compartilhada entre feeds). Mesma lacuna de mecanismo do IDV(8) — precisa de canal cúbico com
`time`, não cabe numa função pura. Ver `docs/05-disturbios.md`.
*/
