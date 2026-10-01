# Distúrbios do TEP (IDV 1–20)

Referência: Downs & Vogel (1993), Table 8 (`docs/disturbios_paper.png`). Cada IDV é ativado via
`active_idv: vec![N]` em `main.rs`.

**Diretriz de implementação (2026-09-19, decisão do usuário, ver #72) — seguir `teprob.f`
estritamente: onde o FORTRAN implementa algo, implementamos igual; onde não implementa, não
implementamos.** IDV(1)–(15) abaixo conferem exatamente com a Table 8 do paper original (variável +
tipo). IDV(16)–(20) são listados no paper como "Unknown"/"Unknown" — mas o `teprob.f` de referência
não os deixa igualmente em aberto: ele dá fórmula concreta pra 16/17/18/20 (mecânica de canal
cúbico/pulso, igual a IDV(1)-(13) — ver `state.rs`: canal 8→UAC, canal 9→QUR, canal 10→QUS, canal
11→coeficiente de vazão) e NENHUMA fórmula pra 19 (nunca referenciado em `IDVWLK` nem em lugar
nenhum do arquivo). A descrição anterior desta seção pra 16-20 ("válvula travada" em vários XMVs)
não vinha do paper nem do `teprob.f` — era uma extrapolação não citada de uma sessão anterior.
Corrigida abaixo: 16/17/18/20 agora refletem a mecânica real de `teprob.f`; 19 fica marcado como
não-implementado; 14/15 continuam "válvula travada" (isso SIM está no paper, Table 8), mas como
categoria de mecanismo estruturalmente diferente — ver nota na seção de IDV(14)/(15) abaixo.

## Onde cada distúrbio entra no código (atualizado em 2026-09-30, pós-migração pra `#[task]`/`need::`/`offer::` e renumeração de streams — issue #73)

Levantamento refeito contra o código Rust atual, linha a linha. Só o IDV(1) está implementado; os
demais são o LOCAL exato onde cada um entraria, não implementação.

| IDV    | Mecanismo                                        | Local exato                                                                             | Nota                                                                                                                            |
| ------ | ------------------------------------------------ | ---------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| **1**  | A/C ratio, stream 4                              | [src/disturbance/idv1.rs:18-33](../src/disturbance/idv1.rs#L18-L33) | **Já implementado**, pelo mecanismo de INTERCEPTAÇÃO (nota logo abaixo da tabela). Intercepta direto a chave pública que [units/feed.rs:46](../src/units/feed.rs#L46) publica e [units/stripper.rs:55](../src/units/stripper.rs#L55) lê — nenhum dos dois sabe que o distúrbio existe |
| **2**  | B composition, stream 4                          | [units/feed.rs:46](../src/units/feed.rs#L46) | mesmo ponto de interceptação do IDV(1) (`flows.stream4_composition`), componente B em vez de A/C |
| **3**  | D feed temperature, step                         | [units/feed.rs:15](../src/units/feed.rs#L15) (`FEED_TEMPERATURE`), consumida em [units/compressor.rs:89](../src/units/compressor.rs#L89) | `FEED_TEMPERATURE` é uma constante ÚNICA reusada por D/E/A/A&C — perturbar só D exige separá-la por feed primeiro |
| **4**  | Reactor CW inlet temp, step                      | [units/reactor.rs:116](../src/units/reactor.rs#L116) (`let twr = REACTOR_COOLING_WATER_RETURN;`) | sem efeito possível hoje — `twr` é constante congelada desde o Exp 24, não depende de `tcwr` nenhum; ver nota abaixo |
| **5**  | Condenser CW inlet temp, step                    | [units/separator.rs:68-69](../src/units/separator.rs#L68-L69) | mesma lacuna estrutural do IDV(4), lado separador |
| **6**  | A feed loss                                      | [units/feed.rs:36](../src/units/feed.rs#L36) (`a_feed_flow`) | fechar a válvula de A é interceptar essa vazão |
| **7**  | C header pressure loss, stream 4                 | [units/feed.rs:41](../src/units/feed.rs#L41) (`ac_feed_flow`) | reduz a vazão combinada A&C inteira, não só C isoladamente — mesma limitação de sempre |
| **8**  | A/B/C composition random, stream 4               | [units/feed.rs:46](../src/units/feed.rs#L46) | mesmo ponto de IDV(1)/(2), perfil aleatório em vez de step |
| **9**  | D feed temperature random                        | [units/feed.rs:15](../src/units/feed.rs#L15), consumida em [units/compressor.rs:89](../src/units/compressor.rs#L89) | mesmo ponto e mesma ressalva do IDV(3) |
| **10** | C feed temperature random, stream 4              | [units/feed.rs:15](../src/units/feed.rs#L15), consumida em [units/stripper.rs:109](../src/units/stripper.rs#L109) | mesmo `FEED_TEMPERATURE` compartilhado, lado A&C |
| **11** | Reactor CW inlet temp random                     | [units/reactor.rs:116](../src/units/reactor.rs#L116) | mesmo ponto e mesma lacuna do IDV(4) |
| **12** | Condenser CW inlet temp random                   | [units/separator.rs:68-69](../src/units/separator.rs#L68-L69) | mesmo ponto do IDV(5) |
| **13** | Reaction kinetics R1F/R2F                        | [units/reactor.rs:8-9](../src/units/reactor.rs#L8-L9) (constantes), usadas em [L21](../src/units/reactor.rs#L21) e [L32](../src/units/reactor.rs#L32) (dentro de `REACTIONS`) | `REACTION_FACTOR_1/2_NOMINAL` |
| **14** | Reactor CW valve sticking                        | [src/actuators/reactor_cooling_water.rs](../src/actuators/reactor_cooling_water.rs) (arquivo inteiro) | sem ponto de interceptação de `write()` hoje — mecanismo não existe ainda |
| **15** | Condenser CW valve sticking                      | [src/actuators/condenser_cooling_water.rs](../src/actuators/condenser_cooling_water.rs) (arquivo inteiro) | mesma observação do IDV(14) |
| **16** | UAC — condenser heat transfer coef.              | [units/stripper.rs:92](../src/units/stripper.rs#L92) (`condenser_ua`) | — |
| **17** | QUR — reactor heat removal                       | [units/reactor.rs:118](../src/units/reactor.rs#L118) (`* (1.0 - 0.35 * 0.0)`) | placeholder já existe |
| **18** | QUS — separator heat removal                     | [units/separator.rs:68](../src/units/separator.rs#L68) (`* (1.0 - 0.25 * 0.0)`) | placeholder já existe |
| **19** | —                                                 | —                                                                                          | não implementar (sem fórmula em `teprob.f`)                                                                                     |
| **20** | Coeficiente de vazão reator→separador             | [units/reactor.rs:101](../src/units/reactor.rs#L101) (`* (1.0 - 0.25 * 0.0)`) | placeholder já existe |

**Mecanismo de INTERCEPTAÇÃO (IDV(1), 2026-10-01, issue #73) — como a maioria dos IDVs acima deveria ser implementada.** `#[monjolo::disturbance(key = "...", intercepts = "...", [components = [...], species = ...])]`, mesmo padrão de `#[actuator]`/`#[sensor]`/`#[controller]`: um struct próprio por IDV, SEM campos (`struct Idv1;`), e um `impl` à parte com um método convencional, `disturb(&self)` — sem parâmetro nenhum, lê via `self.raw()`/`self.active()` (getters gerados), igual `dynamics(&self)`/`control(&self)` já fazem nos outros três. Não vira uma tarefa paralela que lê um nominal e reoferta um público — intercepta a leitura de QUALQUER `need::` pendente pra uma chave já publicada por outra coisa, resolvido uma única vez em `StateRegistry::resolve()` (não por tick, não por lookup em mapa a cada leitura; `disturb()` só roda quando `active() != 0.0`, decidido ANTES de chamar). Quem publica (`offer::`) e quem consome (`need::`) continuam sem saber que o distúrbio existe — a chave pública nunca muda de nome, não existe indireção `"_nominal"`. O struct também se cataloga sozinho como `Actuator` sob `key` — mesma exposição OPC-UA automática de sempre. Ver `monjolo-macros/macros/lib.rs::disturbance` e `monjolo/state_registry.rs` (`DisturbanceInterceptor`, `Proxy::intercept`, `StateRegistry::register_disturbance`).

---

## IDV(1) — Step: razão A/C no feed combinado (stream 4)
A fração molar de A na alimentação A&C muda em step, mantendo B constante. Altera diretamente a estequiometria da reação (A+C→G, A+C→H). Aumenta a taxa de geração de gás e pressão no reator. **Testado no Exp 11 (Kp=0.1, ISD em 2h) e Exp 12 (Kp=1.0, pendente).**

## IDV(2) — Step: composição de B no feed (stream 4)
A fração molar do inerte B aumenta em step na alimentação A&C. B não reage, acumula no loop de reciclo e é removido apenas pelo purge. Eleva o inventário de inerte, aumenta pressão e reduz concentração de reagentes no reator.

## IDV(3) — Step: temperatura do D feed (stream 2)
Temperatura de alimentação de D sobe em step. D é alimentado como líquido; temperatura mais alta muda o enthalpy de entrada e a taxa de vaporização dentro do reator. Efeito moderado sobre temperatura e pressão do reator.

## IDV(4) — Step: temperatura de entrada da água de resfriamento do reator (+5°C)
A água de resfriamento do reator entra 5°C mais quente, reduzindo a capacidade de remoção de calor. Tende a elevar a temperatura do reator e deslocar o equilíbrio vapor-líquido. **Foi o distúrbio usado nos Exps 2 e 3 deste projeto (inadvertidamente ativo).**

**Estado atual (2026-09-16): sem efeito nenhum.** IDV(4) perturba `tcwr` (temperatura de ENTRADA da
água de resfriamento do reator) — mas desde o Exp 24 (`experimentos.md`), `twr` (temperatura de
RETORNO) é uma constante congelada, `REACTOR_COOLING_WATER_RETURN`, sem depender de `tcwr` nenhum.
A fórmula quase-estática que ligava os dois (`twr` como média ponderada entre `tcwr` e a temperatura
do reator, via `uar`/`fcwr`) só existiu historicamente PRA dar a este IDV algum efeito observável
(introduzida em `123a6a3`, 2026-05-27, especificamente por causa disso) — e foi revertida duas vezes
por instabilidade térmica real (colapso de `twr`→~35°C em 2026-03; `QUR<0` quando `tcr`<~67°C em
2026-06), até ser congelada de vez no Exp 24. Dar efeito real a este IDV exige reintroduzir essa
fórmula — desta vez com uma malha de controle de água de resfriamento do reator adequada, que nenhuma
das três tentativas anteriores tinha. Rastreado na epic #71; não é pra ser resolvido silenciosamente
como "sem efeito, fim de história" só porque as tentativas anteriores desestabilizaram o reator.

## IDV(5) — Step: temperatura de entrada da água de resfriamento do condensador (+5°C)
Mesmo mecanismo do IDV(4), mas no condensador do separador. Reduz a condensação no separador, aumenta a fração de vapor no reciclo e eleva a carga sobre o compressor.

## IDV(6) — Step: perda total do A feed (stream 1)
Válvula do A feed fecha completamente. Sem A no reator, a reação para gradualmente. O inventário líquido cai (sem produto G/H sendo gerado) e a composição do loop muda drasticamente. Distúrbio severo — a planta sem controle adequado atinge ISD por nível baixo.

## IDV(7) — Step: queda de pressão no header de C (stream 4)
A pressão de fornecimento de C cai, reduzindo o fluxo de C para o reator. Efeito similar ao IDV(6) para C: menos reagente disponível, reação desbalanceada. Menos severo que o IDV(6) porque a redução é parcial.

## IDV(8) — Aleatório: variação na composição A/B/C do feed (stream 4)
Ruído randômico contínuo na composição da alimentação combinada. Mais difícil de rejeitar do que distúrbios em step porque não tem ponto de operação estacionário alternativo — exige malhas de controle robustas a variação persistente.

## IDV(9) — Aleatório: variação na temperatura do D feed (stream 2)
Ruído randômico na temperatura de entrada de D. Efeito entálpico contínuo sobre o reator. Geralmente mais brando que os distúrbios de composição porque a entalpia de D é uma perturbação de segundo ordem.

## IDV(10) — Aleatório: variação na temperatura do C feed (stream 4)
Ruído randômico na temperatura de C na alimentação combinada A&C. Análogo ao IDV(9) mas para o outro reagente gasoso principal.

## IDV(11) — Aleatório: variação na temperatura de entrada da água de resfriamento do reator
Flutuação contínua na temperatura de entrada do CW do reator. Torna o controle de temperatura do reator inerentemente mais difícil — o controlador de CW precisa compensar uma perturbação de entrada variável.

**Estado atual (2026-09-16): sem efeito nenhum, mesmo motivo do IDV(4)** — mesma variável (`tcwr`),
mesma fórmula `twr` congelada sem dependência dela. Ver nota em IDV(4) acima.

## IDV(12) — Aleatório: variação na temperatura de entrada da água de resfriamento do condensador
Análogo ao IDV(11) para o condensador. Afeta a eficiência de separação e a temperatura do separador (XMEAS(11)).

## IDV(13) — Deriva lenta: cinética de reação
A constante de velocidade da reação deriva lentamente ao longo do tempo (horas). Simula envelhecimento de catalisador ou mudança de condições de processo. Difícil de detectar sem instrumentação adequada; a planta opera aparentemente normal e só falha em horizonte longo.

## IDV(14) — Válvula travada: CW do reator (XMV(10))
A válvula de água de resfriamento do reator trava em sua posição atual. O controlador de temperatura (se existir) perde autoridade. A temperatura do reator passa a derivar conforme o calor de reação acumula sem ser removido.

## IDV(15) — Válvula travada: CW do condensador (XMV(11))
A válvula de resfriamento do condensador trava. A capacidade de condensação no separador fica fixa independentemente da demanda. Com variações de carga, o separador superaquece ou superesfria.

**Mecanismo de IDV(14)/(15) — categoria estruturalmente diferente de todos os outros IDVs desta
lista.** `teprob.f` linha 97 diz explicitamente que IDV(14)-(20) "do NOT require coupling" na
física — nenhuma fórmula em `TEFUNC` os referencia. Pra 14/15 especificamente, isso significa: não
é um VALOR sendo perturbado (não há canal cúbico, não há `TESUB8`/`IDVWLK` envolvido), é o
`Actuator` correspondente que precisa parar de responder a `write()` — a posição da válvula
congela no valor de quando o distúrbio foi ativado, e qualquer comando novo do controlador é
ignorado até o distúrbio ser desativado. Isso não se implementa como "mais um canal de
distúrbio" — é um comportamento de ATUADOR (um decorator/wrapper sobre `Actuator::write()`), não
do componente `Disturbance`. IDV(16)-(20) abaixo, apesar de também estarem na faixa "Unknown" do
paper, NÃO seguem este mecanismo — `teprob.f` os implementa (onde implementa) como perturbação de
valor, igual a IDV(1)-(13).

## IDV(16) — Aleatório (canal cúbico contínuo): coeficiente de troca térmica do condensador (UAC)
`teprob.f`: `UAC = VPOS(9)*VRNG(9)*(1 + TESUB8(9,TIME))/100` — perturbação multiplicativa sobre o
coeficiente de troca térmica do condensador, derivado da posição de válvula. Canal 8 (0-indexado)
de `TepDisturbanceState`, mesma mecânica de canal cúbico contínuo (Block 9) usada por IDV(1)-(3)/
(8)-(10). Sem consumidor no port Rust ainda (`UAC`/condensador não publica esse coeficiente hoje).

## IDV(17) — Aleatório (canal de pulso): remoção de calor do reator (QUR)
`teprob.f`: `QUR = UAR*(TWR-TCR)*(1 - 0.35*TESUB8(10,TIME))` — perturbação multiplicativa sobre a
taxa de remoção de calor do reator. Canal 9 (0-indexado), mecânica de pulso/duração aleatória
(Block 10, diferente do canal cúbico contínuo de 16). **Já tem um placeholder no código** —
`reactor.rs`, `heat_exchange()`: `uar * (twr - reactor_temperature) * (1.0 - 0.35 * 0.0)`, o `0.0`
é onde este canal entra.

## IDV(18) — Aleatório (canal de pulso): remoção de calor do separador/condensador (QUS)
`teprob.f`: `QUS = UAS*(TWS-TST(8))*(1 - 0.25*TESUB8(11,TIME))` — mesma mecânica de IDV(17)
(Block 10, canal 10), aplicada à troca térmica do separador em vez do reator.

## IDV(19) — Não implementado (sem fórmula em `teprob.f`)
Nunca referenciado em `IDVWLK` nem em nenhuma fórmula de `TEFUNC` no `teprob.f` de referência —
ao contrário de 16/17/18/20, que o FORTRAN de fato implementa apesar de rotulados "Unknown" no
paper, 19 fica genuinamente sem definição em nenhuma fonte primária disponível. Diretriz (2026-09-19):
não implementar — não inventar um mecanismo pra ele.

## IDV(20) — Aleatório (canal de pulso): coeficiente de vazão pro separador
`teprob.f`, Block 23: multiplicador `(1 - 0.25*TESUB8(12,TIME))` sobre o cálculo de vazão de vapor
reator→separador. Canal 11 (0-indexado), mecânica de pulso (Block 10, igual 17/18). **Já tem um
placeholder no código** — `reactor.rs`, `flow_to_separator()`: `... * (1.0 - 0.25 * 0.0) / mol_weight`,
o `0.0` é onde este canal entra.
