/* tep/disturbance/idv15.rs */

/* IDV(15) — Válvula travada: CW do condensador, XMV(11) (Table 8, Downs & Vogel 1993).
A válvula de resfriamento do condensador trava. A capacidade de condensação no separador fica fixa
independentemente da demanda. Com variações de carga, o separador superaquece ou superesfria.

CASCA — ainda não implementado. Mesma observação estrutural do IDV(14) (ver `idv14.rs`): é
comportamento de ATUADOR, não um método deste `impl Disturbances` — vai morar em
`src/actuators/condenser_cooling_water.rs`, mecanismo ainda não existe lá. Ver
`docs/05-disturbios.md`.
*/
