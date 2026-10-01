/* tep/disturbance/idv3.rs */

/* IDV(3) — Step: temperatura do D feed (stream 2) (Table 8, Downs & Vogel 1993).
Temperatura de alimentação de D sobe em step. D é alimentado como líquido; temperatura mais alta
muda o enthalpy de entrada e a taxa de vaporização dentro do reator. Efeito moderado sobre
temperatura e pressão do reator.

CASCA — ainda não implementado. Ponto de interceptação: `units/feed.rs` (`FEED_TEMPERATURE`),
consumida em `units/compressor.rs`. `FEED_TEMPERATURE` é uma constante ÚNICA reusada por D/E/A/A&C
— perturbar só D exige separá-la por feed primeiro. Ver `docs/05-disturbios.md`.
*/
