/* tep/sensors/mod.rs */

/** Sensores da planta, um arquivo por sensor (mesma convenção de actuators/) — todos usam
`#[sensor(...)]` (`monjolo-macros`): cada um se auto-registra via `inventory::submit!` escondido;
nenhum `build_tep()` (nem `main()`) precisa conhecer nenhum destes tipos.

Escopo representativo por enquanto (mesmo critério já usado pros XMEAS antes da fusão em
Composite): só as grandezas que já têm chave real publicada por Reactor/Separator/Stripper/
Compressor hoje. A maioria dos 41 XMEAS clássicos depende de Flows/Heat/Measured pra existir,
ainda `todo!()` — pressão/temperatura dos 4 subsistemas já são termodinâmica de verdade, então
entram; "nível" (%) não, porque só existe volume
(`separator.liquid_volume`/`stripper.liquid_volume`), grandeza fisicamente diferente, não um
substituto direto.

Todos usam `Ideal` (sem ruído) por enquanto — `#[sensor(...)]` ainda não aceita escolher
`Noisy`/`Hysteresis`; os desvios-padrão reais por variável estão documentados em
`docs/06-ruidos.md`, pra quando isso entrar em escopo.
*/

mod compressor_pressure;
mod reactor_pressure;
mod reactor_temperature;
mod separator_pressure;
mod separator_temperature;
mod stripper_temperature;

pub use compressor_pressure::CompressorPressure;
pub use reactor_pressure::ReactorPressure;
pub use reactor_temperature::ReactorTemperature;
pub use separator_pressure::SeparatorPressure;
pub use separator_temperature::SeparatorTemperature;
pub use stripper_temperature::StripperTemperature;
