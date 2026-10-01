/* tep/disturbance/idv2.rs */

/* IDV(2) — Step: composição de B no feed combinado (stream 4) (Table 8, Downs & Vogel 1993).
A fração molar do inerte B aumenta em step na alimentação A&C. B não reage, acumula no loop de
reciclo e é removido apenas pelo purge. Eleva o inventário de inerte, aumenta pressão e reduz
concentração de reagentes no reator.

CASCA — ainda não implementado. Mesmo ponto de interceptação do IDV(1) (`flows.stream4_composition`,
componente B em vez de A/C) — ver `idv1.rs`. Ver `docs/05-disturbios.md`.
*/
