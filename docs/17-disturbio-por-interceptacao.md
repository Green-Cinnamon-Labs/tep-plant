# Distúrbio por interceptação — resumo do que mudou (2026-10-01, issue #73)

- Novo mecanismo, no `monjolo`: `StateRegistry` ganhou `register_disturbance(keys, active, transform)` — registra um grupo de chaves que passa a ser interceptado.
- `resolve()` (chamado uma única vez no bootstrap) resolve os índices do grupo e grava `Some(Intercept)` direto em cada `Proxy` pendente cuja chave casa — nenhum lookup em mapa a cada leitura.
- `Proxy::get()` passa a checar essa `Option`: se o comando `active` estiver ligado, lê os valores crus dos "vizinhos" do grupo e aplica `transform(active, vizinhos)` no lugar do valor cru; desligado ou sem distúrbio registrado, devolve o valor cru igual a sempre.
- Quem publica (`offer::`) e quem consome (`need::`) continuam sem saber que o distúrbio existe — nenhuma mudança em `#[task]`, `need::` ou `offer::`.
- Novo atributo de macro: `#[disturbance(key = "...", intercepts = "...", components = [...])]`, sozinho num método (sem `#[need]`/`#[offer]` empilhado) — gera a fiação inteira: o método vira uma função pura, sem `&self`, catalogada como `Actuator` sob `key` (exposição OPC-UA automática, igual a qualquer outro atuador), e chama `register_disturbance` no `construct()`.
- O mecanismo antigo (`#[disturbance(key=...)]` empilhado com `#[need]`/`#[offer]`, que lê um "nominal" e reoferta um "público") continua existindo, intacto, por compatibilidade — não foi removido, só deixou de ser o caminho recomendado.
- `tep-plant::disturbance::idv1::Disturbances::idv1` foi migrado pro mecanismo novo: intercepta direto `flows.stream4_composition.*`, a chave PÚBLICA que `Stripper` já lia.
- `tep-plant::units::feed::Feed::ac_feed_composition` voltou a oferecer `flows.stream4_composition` diretamente — a indireção `flows.stream4_composition_nominal` (que só existia pra dar lugar ao distúrbio relé) foi eliminada.
- `tep-plant::units::stripper::Stripper::flash_split` não mudou uma linha — já lia a chave pública, e passou a receber o valor transparentemente interceptado sem saber disso.
- Teste de `idv1.rs` não mudou nenhuma asserção — só a implementação por trás mudou; prova que o comportamento externo (ligado/desligado) é idêntico ao mecanismo antigo.
- Testes novos: 3 em `monjolo::state_registry` (interceptação básica, não afeta o lado que oferece, erro se uma chave do grupo não tem ofertante) + 1 em `monjolo::component` (ponta-a-ponta via `inventory`/macro real, sem `after`/`need` nenhum ligando o distúrbio ao consumidor).
- `docs/05-disturbios.md`: linha exata do IDV(1) atualizada pro arquivo reescrito, mais uma nota curta explicando o mecanismo novo (fica logo depois da tabela).
- Commits: `monjolo@c7208d4` (branch `73-mixture-chemistry`) e `tep-plant@1ecb227` (branch `73-idv1-disturbance`).
