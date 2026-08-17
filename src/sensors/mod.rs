/* tep/sensors/mod.rs */

/** Sensores da planta, um arquivo por sensor (mesma convenção de actuators/) — todos usam
`#[sensor(...)]` (`monjolo-macros`): cada um se auto-registra via `inventory::submit!` escondido;
nenhum `build_tep()` (nem `main()`) precisa conhecer nenhum destes tipos.

Escopo representativo por enquanto: as grandezas termodinâmicas diretas (pressão/temperatura dos 4
subsistemas) mais os 2 níveis (%) que os 3 controladores P clássicos do TEP precisam
(`docs/07-controle.md`) — `xmeas.separator_level`/`xmeas.stripper_level`, publicados por `Measured`
(já convertidos de volume pra %, Block 21 de teprob.f), não `separator.liquid_volume`/
`stripper.liquid_volume` brutos. O resto dos 41 XMEAS clássicos continua fora de escopo por
enquanto — nada ainda os consome.

Todos usam `Ideal` (sem ruído) por enquanto — `#[sensor(...)]` ainda não aceita escolher
`Noisy`/`Hysteresis`; os desvios-padrão reais por variável estão documentados em
`docs/06-ruidos.md`, pra quando isso entrar em escopo.
*/

mod compressor_pressure;
mod reactor_pressure;
mod reactor_temperature;
mod separator_level;
mod separator_pressure;
mod separator_temperature;
mod stripper_level;
mod stripper_temperature;

pub use compressor_pressure::CompressorPressure;
pub use reactor_pressure::ReactorPressure;
pub use reactor_temperature::ReactorTemperature;
pub use separator_level::SeparatorLevel;
pub use separator_pressure::SeparatorPressure;
pub use separator_temperature::SeparatorTemperature;
pub use stripper_level::StripperLevel;
pub use stripper_temperature::StripperTemperature;
