/* tep/disturbance/idv2.rs */

use crate::physics::constants::TEP_SPECIES;
use monjolo::chemistry::Mixture;

/** IDV(2) — Step: composição de B no feed combinado (Table 8, Downs & Vogel 1993; `teprob.f`:
`XST(1,4) -= IDV(2)*2.43719e-3`, `XST(2,4) += IDV(2)*0.005`, C absorve via `XST(3,4) = 1 - XST(1,4)
- XST(2,4)`). A fração molar do inerte B aumenta em step na alimentação A&C — B não reage, acumula
no loop de reciclo e é removido só pelo purge; eleva o inventário de inerte, aumenta pressão e
reduz concentração de reagentes no reator.

Convive com `idv1.rs` de propósito: os dois interceptam a MESMA chave (`flows.stream4_composition`)
e formam uma cadeia (ver `#[monjolo::disturbance]`/`StateRegistry::register_disturbance`), aplicada
em ordem de registro — ligados os dois, o resultado é exatamente a soma do `teprob.f`
(`A = nominal - 0.03·IDV1 - 2.43719e-3·IDV2`), não um apagando o outro. Prova em
`tests/disturbance/idv1_and_idv2.rs`.
*/
#[monjolo::disturbance(
    key = "disturbance.idv2",
    intercepts = "flows.stream4_composition",
    components = ["a", "b", "c", "d", "e", "f", "g", "h"],
    species = TEP_SPECIES
)]
pub struct Idv2;

impl Idv2 {
    fn disturb(&self) -> Mixture<8> {
        let mut composition = self.raw().as_array();
        composition[0] -= 2.43719e-3;
        composition[1] += 0.005;
        composition[2] = 1.0 - composition[0] - composition[1];
        Mixture::new(composition, &TEP_SPECIES)
    }
}
