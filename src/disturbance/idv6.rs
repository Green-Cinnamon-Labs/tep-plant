/* tep/disturbance/idv6.rs */

/* IDV(6) — Step: perda total do A feed (stream 1) (Table 8, Downs & Vogel 1993).
Válvula do A feed fecha completamente. Sem A no reator, a reação para gradualmente. O inventário
líquido cai (sem produto G/H sendo gerado) e a composição do loop muda drasticamente. Distúrbio
severo — a planta sem controle adequado atinge ISD por nível baixo.

CASCA — ainda não implementado. Ponto de interceptação: `units/feed.rs` (`a_feed_flow`) — fechar a
válvula de A é interceptar essa vazão. Ver `docs/05-disturbios.md`.
*/
