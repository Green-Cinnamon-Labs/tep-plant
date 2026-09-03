/* tep/sensors/mod.rs */

/** Sensores da planta, um arquivo por sensor (mesma convenção de actuators/) — todos usam
`#[sensor(...)]` (`monjolo-macros`): cada um se auto-registra via `inventory::submit!` escondido;
nenhum `build_tep()` (nem `main()`) precisa conhecer nenhum destes tipos.

As 22 XMEAS diretas (1-22) mais `status.shutdown_detected` (diagnóstico à parte, fora da contagem
41 XMEAS + 12 XMV) — todas já computadas e publicadas por `dynamics::measured::Measured`, aqui só
expostas via `#[sensor(key=...)]`. As 19 XMEAS de composição (23-41) são publicadas por
`dynamics::reactor_feed_analyzer`/`purge_analyzer`/`product_analyzer` e expostas por sensores
próprios, também aqui.

Nomes seguem `xmeas.<local>.<grandeza>`, onde `<local>` é a STREAM FÍSICA do TEP (não o índice da
XMEAS) pras medições de vazão sem unidade própria, ou o nome do vaso pras medições locais —
verificado contra o cabeçalho de `docs/fortran-original/teprob.f:122-143`.

Todos usam `Ideal` (sem ruído) por enquanto — `#[sensor(...)]` ainda não aceita escolher
`Noisy`/`Hysteresis`; os desvios-padrão reais por variável estão documentados em
`docs/06-ruidos.md`, pra quando isso entrar em escopo.
*/

mod a_feed;
mod ac_feed;
mod compressor_work;
mod d_feed;
mod e_feed;
mod product_component_d;
mod product_component_e;
mod product_component_f;
mod product_component_g;
mod product_component_h;
mod purge_component_a;
mod purge_component_b;
mod purge_component_c;
mod purge_component_d;
mod purge_component_e;
mod purge_component_f;
mod purge_component_g;
mod purge_component_h;
mod purge_rate;
mod reactor_cooling_water_outlet_temperature;
mod reactor_feed_component_a;
mod reactor_feed_component_b;
mod reactor_feed_component_c;
mod reactor_feed_component_d;
mod reactor_feed_component_e;
mod reactor_feed_component_f;
mod reactor_feed_rate;
mod reactor_level;
mod reactor_pressure;
mod reactor_temperature;
mod recycle_flow;
mod separator_cooling_water_outlet_temperature;
mod separator_level;
mod separator_pressure;
mod separator_temperature;
mod separator_underflow;
mod shutdown_detected;
mod stripper_level;
mod stripper_pressure;
mod stripper_steam_flow;
mod stripper_temperature;
mod stripper_underflow;

pub use a_feed::AFeed;
pub use ac_feed::AcFeed;
pub use compressor_work::CompressorWork;
pub use d_feed::DFeed;
pub use e_feed::EFeed;
pub use product_component_d::ProductComponentD;
pub use product_component_e::ProductComponentE;
pub use product_component_f::ProductComponentF;
pub use product_component_g::ProductComponentG;
pub use product_component_h::ProductComponentH;
pub use purge_component_a::PurgeComponentA;
pub use purge_component_b::PurgeComponentB;
pub use purge_component_c::PurgeComponentC;
pub use purge_component_d::PurgeComponentD;
pub use purge_component_e::PurgeComponentE;
pub use purge_component_f::PurgeComponentF;
pub use purge_component_g::PurgeComponentG;
pub use purge_component_h::PurgeComponentH;
pub use purge_rate::PurgeRate;
pub use reactor_cooling_water_outlet_temperature::ReactorCoolingWaterOutletTemperature;
pub use reactor_feed_component_a::ReactorFeedComponentA;
pub use reactor_feed_component_b::ReactorFeedComponentB;
pub use reactor_feed_component_c::ReactorFeedComponentC;
pub use reactor_feed_component_d::ReactorFeedComponentD;
pub use reactor_feed_component_e::ReactorFeedComponentE;
pub use reactor_feed_component_f::ReactorFeedComponentF;
pub use reactor_feed_rate::ReactorFeedRate;
pub use reactor_level::ReactorLevel;
pub use reactor_pressure::ReactorPressure;
pub use reactor_temperature::ReactorTemperature;
pub use recycle_flow::RecycleFlow;
pub use separator_cooling_water_outlet_temperature::SeparatorCoolingWaterOutletTemperature;
pub use separator_level::SeparatorLevel;
pub use separator_pressure::SeparatorPressure;
pub use separator_temperature::SeparatorTemperature;
pub use separator_underflow::SeparatorUnderflow;
pub use shutdown_detected::ShutdownDetected;
pub use stripper_level::StripperLevel;
pub use stripper_pressure::StripperPressure;
pub use stripper_steam_flow::StripperSteamFlow;
pub use stripper_temperature::StripperTemperature;
pub use stripper_underflow::StripperUnderflow;
