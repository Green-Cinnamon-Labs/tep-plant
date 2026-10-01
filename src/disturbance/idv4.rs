/* tep/disturbance/idv4.rs */

/* IDV(4) — Step: temperatura de entrada da água de resfriamento do reator, +5°C (Table 8, Downs &
Vogel 1993). A água de resfriamento do reator entra 5°C mais quente, reduzindo a capacidade de
remoção de calor. Tende a elevar a temperatura do reator e deslocar o equilíbrio vapor-líquido.
**Foi o distúrbio usado nos Exps 2 e 3 deste projeto (inadvertidamente ativo).**

CASCA — ainda não implementado, E sem efeito possível hoje mesmo se implementado: IDV(4) perturba
`tcwr` (temperatura de ENTRADA da água de resfriamento do reator) — mas desde o Exp 24
(`experimentos.md`), `twr` (temperatura de RETORNO, `units/reactor.rs`,
`REACTOR_COOLING_WATER_RETURN`) é uma constante CONGELADA, sem depender de `tcwr` nenhum. A fórmula
que ligava os dois (via `uar`/`fcwr`) só existiu historicamente pra dar a este IDV algum efeito
observável, e foi revertida duas vezes por instabilidade térmica real — até ser congelada de vez no
Exp 24. Dar efeito real a este IDV exige reintroduzir essa fórmula, desta vez com uma malha de
controle de água de resfriamento do reator adequada (nenhuma das três tentativas anteriores tinha).
Rastreado na epic #71. Ver `docs/05-disturbios.md`.

TODO: reintroduzir `twr = f(tcwr, ...)` com uma malha de controle de água de resfriamento do
reator de verdade — só então implementar o `#[monjolo::disturbance]` deste IDV tem efeito real.
*/
