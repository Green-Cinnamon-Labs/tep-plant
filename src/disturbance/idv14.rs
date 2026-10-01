/* tep/disturbance/idv14.rs */

/* IDV(14) — Válvula travada: CW do reator, XMV(10) (Table 8, Downs & Vogel 1993).
A válvula de água de resfriamento do reator trava em sua posição atual. O controlador de
temperatura (se existir) perde autoridade. A temperatura do reator passa a derivar conforme o calor
de reação acumula sem ser removido.

CASCA — ainda não implementado, E estruturalmente NÃO é um `Disturbances::idv14` como os demais
IDVs deste diretório. `teprob.f` linha 97 diz que IDV(14)-(20) "do NOT require coupling" na física —
não é um VALOR sendo perturbado (sem canal cúbico, sem `TESUB8`/`IDVWLK`), é o `Actuator`
correspondente que precisa parar de responder a `write()`: a posição da válvula congela no valor de
quando o distúrbio foi ativado, e qualquer comando novo do controlador é ignorado até desativar.
Isso é um comportamento de ATUADOR (um decorator/wrapper sobre `Actuator::write()`), não um método
deste `impl Disturbances` — vai morar em `src/actuators/reactor_cooling_water.rs`, mecanismo ainda
não existe lá. Ver `docs/05-disturbios.md`.
*/
