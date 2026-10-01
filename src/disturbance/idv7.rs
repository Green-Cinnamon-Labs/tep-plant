/* tep/disturbance/idv7.rs */

/* IDV(7) — Step: queda de pressão no header de C (stream 4) (Table 8, Downs & Vogel 1993).
A pressão de fornecimento de C cai, reduzindo o fluxo de C para o reator. Efeito similar ao IDV(6)
para C: menos reagente disponível, reação desbalanceada. Menos severo que o IDV(6) porque a redução
é parcial.

CASCA — ainda não implementado. Ponto de interceptação: `units/feed.rs` (`ac_feed_flow`) — reduz a
vazão combinada A&C inteira, não só C isoladamente (mesma limitação de sempre, stream 4 não separa
A de C). Ver `docs/05-disturbios.md`.
*/
