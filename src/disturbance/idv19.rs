/* tep/disturbance/idv19.rs */

/* IDV(19) — Não implementado (sem fórmula em `teprob.f`).
Listado como "Unknown" no paper — mas, ao contrário de 16/17/18/20 (que o FORTRAN de fato
implementa apesar do rótulo), IDV(19) nunca é referenciado em `IDVWLK` nem em nenhuma fórmula de
`TEFUNC` no `teprob.f` de referência. Fica genuinamente sem definição em nenhuma fonte primária
disponível.

CASCA intencionalmente vazia. Diretriz (2026-09-19, issue #72) — seguir `teprob.f` estritamente:
onde o FORTRAN não implementa, não implementamos. Não inventar um mecanismo pra ele. Ver
`docs/05-disturbios.md`.
*/
