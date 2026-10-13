/* Testes da planta avaliada como função (`monjolo::model_function`), sem laço de simulação —
spec-tennessee-eastman#93.
*/

/* Mesmo motivo de `src/main.rs`: sem isto o linker não puxa os `inventory::submit!` da planta, e
`ModelFunction::discovered()` não acharia componente nenhum.
*/
extern crate tennessee_eastman_process;

mod base_case;
