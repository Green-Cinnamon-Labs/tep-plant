/* tep/disturbance/idv1.rs */

use crate::physics::constants::TEP_SPECIES;
use monjolo::chemistry::Mixture;

/** IDV(1) — Step: razão A/C no feed combinado (Table 8, Downs & Vogel 1993; `teprob.f`:
`XST(1,4) -= IDV(1)*0.03`, C absorve a diferença via `XST(3,4) = 1 - XST(1,4) - XST(2,4)`, B
intocado). A fração molar de A na alimentação A&C muda em step, mantendo B constante. Altera
diretamente a estequiometria da reação (A+C→G, A+C→H). Aumenta a taxa de geração de gás e pressão
no reator. **Testado no Exp 11 (Kp=0.1, ISD em 2h) e Exp 12 (Kp=1.0, pendente).**

Implementado pelo mecanismo de INTERCEPTAÇÃO (mesmo padrão de `#[actuator]`/`#[sensor]`/
`#[controller]` — ver `#[monjolo::disturbance]`): intercepta DIRETO a chave pública que
`units::feed::Feed` publica e `units::stripper` consome (`flows.stream4_composition.*`), sem que
nenhum dos dois saiba que isto existe. `disturb(&self)` só roda quando `active() != 0.0` — o
framework já decide isso antes de chamar, então não precisa (nem deve) checar de novo aqui dentro.
*/
#[monjolo::disturbance(
    key = "disturbance.idv1",
    intercepts = "flows.stream4_composition",
    components = ["a", "b", "c", "d", "e", "f", "g", "h"],
    species = TEP_SPECIES
)]
pub struct Idv1;

impl Idv1 {
    fn disturb(&self) -> Mixture<8> {
        let mut composition = self.raw().as_array();
        composition[0] -= 0.03;
        composition[2] = 1.0 - composition[0] - composition[1];
        Mixture::new(composition, &TEP_SPECIES)
    }
}
