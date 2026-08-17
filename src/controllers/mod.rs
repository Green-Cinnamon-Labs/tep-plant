/* tep/controllers/mod.rs */

/** Controllers da planta, um arquivo por controller (mesma convenção de actuators/sensors) — usa
`#[controller(...)]` (`monjolo-macros`): se auto-registra via `inventory::submit!` escondido;
nenhum `build_tep()` (nem `main()`) precisa conhecer o tipo. Cada um declara campos `#[sensor(key =
"...")]`/`#[actuator(key = "...")]` e escreve seu próprio `control(&self)` (chamado por
`evaluate()`, gerado pela macro) — mesmo padrão de `#[actuator(...)]`/`dynamics()`.

Os 3 controladores P clássicos do TEP (Downs & Vogel 1993; `experimentos.md`, Exp 10/11/13),
validados como necessários e suficientes pra manter a planta estável — sem eles, o desbalanço de
massa gasosa e o inventário de líquido dos vasos derivam sem limite (Exp 8/9):
- pressão do reator → purge
- nível do separador → separator underflow
- nível do stripper → stripper product

Uma 4ª malha (nível do reator → A feed) foi tentada e **refutada** no Exp 9 — piorou o desbalanço
em vez de corrigi-lo — de propósito NÃO está implementada aqui.
*/

mod reactor_pressure_control;
mod separator_level_control;
mod stripper_level_control;

pub use reactor_pressure_control::ReactorPressureControl;
pub use separator_level_control::SeparatorLevelControl;
pub use stripper_level_control::StripperLevelControl;
