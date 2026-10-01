# Streams/Flows: por que paramos o trabalho de distúrbios pra discutir isso

Este documento existe porque a conversa sobre "como o IDV(1) deveria alterar a composição do feed A&C" esbarrou num problema mais fundo: **hoje o código não tem o conceito de "uma stream do processo" em lugar nenhum.** O que existe são pedaços soltos — uma vazão aqui, uma composição ali, sem nada que amarre os dois como "isto tudo é a MESMA corrente física." Antes de continuar distúrbio nenhum, precisamos decidir como representar uma stream corretamente, porque é exatamente aí que um distúrbio vai precisar se encaixar depois.

Pra quem está chegando nesta conversa sem contexto: o TEP (Tennessee Eastman Process) é uma planta química simulada com ~13 "correntes" nomeadas (streams) — tubulações por onde vazão de gás/líquido circula entre reator, separador, compressor e coluna de stripping. Cada stream tem uma VAZÃO (quanto material passa por ali, por hora) e uma COMPOSIÇÃO (de quais das 8 substâncias químicas do processo — chamadas A, B, C, D, E, F, G, H — esse material é feito, em fração molar). O código Rust deste projeto (`tep-plant`) reimplementa essa planta usando um framework próprio (`monjolo`) que já foi explicado em outros docs — a parte que interessa aqui é só que cada "unidade" (Reator, Separador, Stripper, Compressor, Feed) é um pedaço de código que PUBLICA valores num registro central (`StateRegistry`) sob um nome (uma "chave", tipo `"flows.stream_flow.3"`) pra que outras unidades leiam esse valor depois, sem se conhecerem diretamente.

---

## O problema concreto que disparou essa parada

Em `units/feed.rs`, a vazão do feed combinado A&C é publicada assim:

```rust
#[offer(key = "flows.stream_flow.3")]
fn ac_feed_flow(&self, position: f64) -> f64 { ... }
```

E a composição do MESMO feed A&C é publicada em outro método, com outro nome, sem relação textual nenhuma com o primeiro:

```rust
#[offer(prefix = "flows.stream4_composition_nominal", components = ["a", ..., "h"])]
fn ac_feed_composition(&self) -> [f64; 8] { ... }
```

São duas coisas que, na vida real, são o MESMO objeto físico (a corrente A&C) — mas no código são dois métodos desconectados, com nomes de chave que nem seguem o mesmo padrão (`stream_flow.3` vs `stream4_composition`). Quando surgiu a pergunta "onde o IDV(1) deveria alterar essa composição?", ficou claro que estávamos prestes a criar um TERCEIRO pedaço solto (`flows.stream4_composition`, a versão "pública" pós-distúrbio) — empilhando gambiarra em cima de uma modelagem que já estava errada. Por isso a pausa: primeiro arruma o alicerce (o que É uma stream, como ela deveria ser representada), só depois volta a discutir onde o distúrbio entra nela.

## O que é, fisicamente, cada stream — e por que isso importa

Fui conferir contra o FORTRAN original (`docs/fortran-original/teprob.f`, a referência que este projeto porta pra Rust) quais streams existem de verdade e o que cada uma carrega. Achado importante: nem toda stream tem os 8 componentes químicos — os feeds PUROS (que entram de fora da planta) têm só 2 ou 3, porque fisicamente são quase-puros; só as streams MISTURADAS (depois que passaram pelo reator ou por um separador) têm as 8 de verdade.

| Stream | O que é                          | Componentes reais            | Por quê                                                                    |
| ------ | -------------------------------- | ---------------------------- | -------------------------------------------------------------------------- |
| 1      | Feed de A                        | A, B (2)                     | é A quase puro, só traço de B                                              |
| 2      | Feed de D                        | B, D (2)                     | é D quase puro, só traço de B                                              |
| 3      | Feed de E                        | E, F (2)                     | é E quase puro, só traço de F                                              |
| 4      | Feed combinado A&C               | A, B, C (3)                  | é o que IDV(1)/(2)/(8) perturbam                                           |
| 6/7/8  | Dentro do loop reator↔compressor | todos os 8                   | depois do reator já existe G/H, e nada foi separado ainda                  |
| 9      | Purga                            | todos os 8                   | tirada do meio do loop de reciclo                                          |
| 10     | Saída de líquido do separador    | todos os 8                   | idem                                                                       |
| 11     | Produto final do stripper        | D, E, F, G, H (5, SEM A/B/C) | é literalmente o trabalho do stripper: remover A/B/C (os leves) do produto |

A regra que sai disso: **cada stream carrega só os componentes que fazem sentido fisicamente pra ela — isso não é uma simplificação de programador, é a física real da planta.** Fingir que todo stream tem os 8 componentes (como o código faz hoje, publicando `[f64; 8]` pra tudo) esconde essa informação real por trás de zeros que nunca mudam.

## Por que não dá pra simplesmente "publicar o Stream como um objeto"

A ideia natural seria: em vez de publicar vazão e composição como duas coisas soltas, publicar UM objeto `Stream4` que já carrega os dois, e quem precisar da composição pega esse objeto e lê `stream.composicao()` — sem precisar saber o nome de nenhuma chave separada.

O obstáculo é técnico: o `StateRegistry` (o "registro central" mencionado lá em cima) só sabe guardar UM NÚMERO (`f64`) por chave nomeada — ele não tem o conceito de "um objeto com vários campos" guardado sob uma chave só. Toda a maquinaria de `#[offer(...)]`/`#[need(...)]` que já existe no projeto (as anotações que aparecem em cima dos métodos) foi construída em cima dessa ideia de "cada atributo publica um número, ou uma lista de números" — nunca "um objeto." Pra publicar um `Stream4` de verdade como objeto único, seria preciso ensinar essa maquinaria a entender "isto é um struct com campos nomeados, quebra cada campo em uma chave própria" — o que não existe hoje.

## Os dois caminhos possíveis

**Caminho simples (recomendado pra agora):** continuamos publicando vazão e composição como chaves separadas — mas com nomes CONSISTENTES (ex.: `flows.stream4.flow`, `flows.stream4.composition.a`, `.b`, `.c`, todos sob o mesmo prefixo `flows.stream4`, em vez dos nomes desencontrados de hoje) — e cada método que PRECISA desses dados monta, logo no início do próprio corpo, um pequeno struct Rust local (`Stream4 { flow, composition }`) só pra facilitar a leitura (`stream.composicao()[0]` em vez de um parâmetro solto). Nenhuma mudança no framework — só disciplina de nome e um struct de conveniência por stream, escrito à mão. Cada stream declara só os componentes que tem de verdade (2, 2, 2, 3, ou os 8 — conforme a tabela acima), nunca os 8 por padrão.

**Caminho completo (mais correto, mais trabalho):** ensinar a macro do framework (o código que processa as anotações `#[need(...)]`/`#[offer(...)]`) a entender um novo formato, `#[need(stream = "flows.stream4")]`, que devolveria automaticamente um objeto pronto com `.flow()`/`.composition()` — sem precisar de dois atributos separados nem do struct de conveniência escrito à mão. Isso é mais fiel à ideia original ("o Stream é um objeto único"), mas exige mexer de novo no gerador de código do framework (`monjolo-macros`), que já foi alterado várias vezes nesta mesma conversa.

## Onde isso deixa o trabalho de distúrbios

Parado, de propósito. O `IDV(1)` (e qualquer outro distúrbio de composição/temperatura) só volta a ser discutido depois que a forma da stream estiver decidida e estável — não faz sentido desenhar "onde o distúrbio entra" em cima de uma modelagem que ainda pode mudar de formato. Nenhum código de `disturbance/` foi tocado desde que esta pausa começou.
