/* tep/disturbance/idv1.rs */

use super::Disturbances;

/** IDV(1) — Step: razão A/C no feed combinado (Table 8, Downs & Vogel 1993; `teprob.f`:
`XST(1,4) -= IDV(1)*0.03`, C absorve a diferença via `XST(3,4) = 1 - XST(1,4) - XST(2,4)`, B
intocado). A fração molar de A na alimentação A&C muda em step, mantendo B constante. Altera
diretamente a estequiometria da reação (A+C→G, A+C→H). Aumenta a taxa de geração de gás e pressão
no reator. **Testado no Exp 11 (Kp=0.1, ISD em 2h) e Exp 12 (Kp=1.0, pendente).**

Implementado pelo mecanismo de INTERCEPTAÇÃO (ver `Disturbances`, em `mod.rs`): intercepta DIRETO a
chave pública que `units::feed::Feed` publica e `units::stripper` consome
(`flows.stream4_composition.*`), sem que nenhum dos dois saiba que isto existe.

`pub`: `tests/disturbance/idv1.rs` (binário de teste SEPARADO, outro crate) precisa referenciar
`Disturbances::idv1` diretamente pra forçar o linker a manter este arquivo compilado — sem isso,
nada no teste tocaria este módulo e o `inventory::submit!` que a macro gera ao lado nunca entraria
no link (mesmo truque de `tests/units/feed.rs` com `Feed::new`).
*/
#[monjolo::tasks]
impl Disturbances {
    #[disturbance(
        key = "disturbance.idv1",
        intercepts = "flows.stream4_composition",
        components = ["a", "b", "c", "d", "e", "f", "g", "h"]
    )]
    pub fn idv1(active: f64, raw: &[f64]) -> Vec<f64> {
        let mut composition = raw.to_vec();
        if active != 0.0 {
            composition[0] -= 0.03;
            composition[2] = 1.0 - composition[0] - composition[1];
        }
        composition
    }
}
