# A planta como função

Este documento descreve como avaliar a planta num ponto qualquer, fora do laço de simulação, e registra a validação dessa avaliação no caso base de Downs & Vogel. Faz parte da linha de trabalho da spec-tennessee-eastman#91 e só existe nas branches dessa linha.

## O que é

Um otimizador não precisa da planta rodando no tempo. Ele precisa perguntar "neste ponto, quanto valem as derivadas e as medições?" e receber sempre a mesma resposta para a mesma pergunta.

O `monjolo::model_function::ModelFunction` faz isso sobre o `StateRegistry`: escreve valores em chaves, avalia o modelo e lê outras chaves. Estado, derivada, medição, posição de válvula e liga/desliga de distúrbio já são chaves do registry, então não há interface nova a declarar.

```rust
let config = Snapshot::from_file("docs/cases/te_mode1_initial_state.toml")?;
let function = ModelFunction::discovered(&config)?;

let result = function.evaluate(
    &[("valve.purge.position", 30.0)],
    &["xmeas.reactor.pressure", "reactor.state.vapor_a.derivative"],
)?;
```

## Regras de uso

- **Toda chave pode ser lida.** `function.keys()` lista as 253 chaves; `function.state_keys()` lista os 50 estados.
- **Escrever só tem efeito em estado ou em liga/desliga de distúrbio.** Qualquer outra chave é recalculada pela própria avaliação.
- **Cada avaliação parte do ponto base** (o conteúdo do TOML de configuração) e aplica as escritas por cima. Uma escrita não sobrevive para a chamada seguinte.
- **Os controladores e os sensores ficam de fora.** Quem chama escreve direto as 12 posições de válvula. As medições são lidas das chaves `xmeas.*`, que não têm ruído; o ruído só existe dentro dos sensores.
- **A derivada de uma válvula não tem uso aqui.** Ela mede a distância até o comando guardado no atuador, que nesse modo fica parado no valor do TOML. Em regime, posição e comando são iguais.

## Os 50 estados

São os mesmos de Downs & Vogel: 9 de cada vaso (reator, separador, stripper e compressor), as 2 temperaturas da água de resfriamento e as 12 válvulas. As temperaturas da água voltaram a ser estados na spec-tennessee-eastman#90; ver [12-reator.md](12-reator.md) e [13-separador.md](13-separador.md).

## Validação no caso base (2026-10-13)

O teste está em `tests/model_function/base_case.rs` e usa `docs/cases/te_mode1_initial_state.toml`, que são os valores de `TEINIT` do `teprob.f`. A referência é Ricker (1995), Tabelas 2 e 4, coluna "Base case".

### As derivadas ficam próximas de zero

O caso base publicado é um ponto de regime, e a planta confirma isso.

| Grupo | Maior módulo da derivada |
|---|---|
| Separador (9 estados) | 3,2e-6 |
| Stripper (9 estados) | 4,7e-8 |
| Compressor (9 estados) | 5,3e-6 |
| Reator (9 estados) | 9,3e-4 |
| Água de resfriamento do reator | 4,4e-7 °C/h |
| Água de resfriamento do condensador | 3,3e-7 °C/h |

As duas derivadas da água de resfriamento são a diferença entre duas parcelas de cerca de 1700 °C/h cada, então fecharem em 1e-7 confirma as constantes das equações restauradas.

No reator o resíduo é maior e tem um padrão: A, C e D caem e G sobe em 6,6e-4 kmol/h; A, C e E caem e H sobe em 2,7e-4 kmol/h. É um pequeno excesso das duas reações principais, da ordem de 6e-6 da taxa de reação. O maior valor relativo ao próprio estado é 1,6e-3 por hora, no líquido D, que tem só 0,42 kmol. Não foi investigado se o FORTRAN dá o mesmo resíduo nesse ponto.

### As medições batem com a Tabela 2

As 41 XMEAS, arredondadas para as casas que a tabela imprime, dão o número da tabela. Alguns valores:

| Medição | Planta | Tabela 2 |
|---|---|---|
| Pressão do reator, kPa | 2705,000 | 2705 |
| Nível do reator, % | 75,000 | 75 |
| Temperatura do reator, °C | 120,400 | 120,4 |
| Vazão de purga, kscmh | 0,3371 | 0,337 |
| Vazão de produto, m³/h | 22,949 | 22,95 |
| Trabalho do compressor, kW | 341,43 | 341 |
| Vapor do stripper, kg/h | 230,31 | 230 |
| Água do reator, °C | 94,599 | 94,6 |
| Água do condensador, °C | 77,297 | 77,3 |

### O custo bate com a Tabela 4

Calculado pela equação 3 do artigo:

| Parcela, $/h | Planta | Tabela 4 |
|---|---|---|
| Purga | 114,730 | 114,71 |
| Produto | 30,271 | 30,28 |
| Compressor | 18,301 | 18,30 |
| Vapor | 7,324 | 7,32 |
| Total | 170,626 | 170,61 |

### A função não guarda memória

Dois testes, comparando todas as 253 chaves bit a bit:

- **A, B, A:** o caso base, depois um ponto com cada estado deslocado em até 15 % (a pressão do reator muda em mais de 100 kPa), depois o caso base de novo. O resultado do caso base é idêntico antes e depois.
- **Instância reaproveitada contra instância nova:** doze pontos variados avaliados em sequência numa mesma instância dão, cada um, exatamente o que uma instância recém-criada dá.

Além disso, a montagem do `ModelFunction` recusa um modelo em que avaliar duas vezes seguidas o mesmo ponto muda algum valor.
