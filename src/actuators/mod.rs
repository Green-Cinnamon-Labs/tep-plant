/* tep/actuators/mod.rs */

/** Os 12 atuadores físicos da planta (XMV-1 a XMV-12), um arquivo por atuador — mesma convenção de
`reactor.rs`/`separator.rs`/`stripper.rs`/`compressor.rs` (um por unidade). Todos os 12 usam
`#[actuator(...)]` (`monjolo-macros`) — cada um se auto-registra via `inventory::submit!` escondido
gerado pela macro; nenhum `build_tep()` (nem `main()`) precisa conhecer nenhum destes 12 tipos.
`pub use` abaixo é só pra quem quiser um handle direto (testes, depuração) — não é o mecanismo que
liga cada atuador na simulação real, que é inteiramente `inventory`.

τ vem de VTAU(n) em teprob.f, cross-checado contra as equações físicas que consomem VPOS(I)
(`docs/_deprecated_1.rs` rotula XMV-9/XMV-11 trocados — ver histórico deste diretório, antes
`actuators.rs`).
*/

mod agitator;
mod compressor_recycle;
mod condenser_cooling_water;
mod feed_a;
mod feed_ac;
mod feed_d;
mod feed_e;
mod purge;
mod reactor_cooling_water;
mod separator_underflow;
mod stripper_product;
mod stripper_steam;

pub use agitator::Agitator;
pub use compressor_recycle::CompressorRecycle;
pub use condenser_cooling_water::CondenserCoolingWater;
pub use feed_a::FeedA;
pub use feed_ac::FeedAc;
pub use feed_d::FeedD;
pub use feed_e::FeedE;
pub use purge::Purge;
pub use reactor_cooling_water::ReactorCoolingWater;
pub use separator_underflow::SeparatorUnderflow;
pub use stripper_product::StripperProduct;
pub use stripper_steam::StripperSteam;
