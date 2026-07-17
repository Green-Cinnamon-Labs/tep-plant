# Tennessee Eastman CPS — Premissas

## Sobre válvulas e Atuadores

No artigo e modelagem inical, VPOS é a posição da válcula (0-100%), e o fluxo é calculado a partir dela: $$ FTM = VPOS * \frac{VRNG}{100} $$ . Entrantando, as EDO's usam FTM (fluxo), não as VPOS diretamente. Mas, VPOS em si é um estado dinâmico — a válvula tem inércia, não pula instantaneamente para o valor comandado. Então existe uma EDO que diz "como VPOS muda no tempo em função do comando XMV". No artigo, portanto, o fluxo é uma variável intermediária calculada, não um estado.

No nosso caso, o FTM (fluxo) irá substituir VPOS no vetor de estados YY. Faremos esse FTM será calculado externamente por sua própria dinâmica e atualizado no YY. Através de um comando externo XMV, a posição da válvula (VPOS) muda, o que implica em um novo FTM que é armazenado em YY.

Haverá um barramento que carrega o sinal XMV capaz de atualizar a válvula, seguindo uma arquitetura TX/RX e baseado no protocolo Modbus.

## Sobre distúrbios

Os distúrbios IDV são flags inteiros (0 ou 1) que modulam os canais de perturbação internos do modelo. Eles não são estados dinâmicos — são entradas externas que alteram parâmetros calculados (composição do feed, temperaturas de entrada, fatores de reação). Os canais de perturbação usam splines cúbicas com segmentos atualizados no tempo, com seeds determinísticos. Com todos os IDV desligados e todos os canais em `s_zero`, o modelo é completamente determinístico e os distúrbios são zero.

---

## Sobre o Vetor de Estados

O vetor de estados YY tem 50 posições no modelo Rust, mapeando diretamente o layout do FORTRAN TEINIT:

| Índices (0-based) | Variável      | Descrição                                     |
|-------------------|---------------|-----------------------------------------------|
| 0–7               | UCVR / UCLR   | Conteúdos molares por componente no reator    |
| 8                 | ETR           | Energia interna total do reator               |
| 9–16              | UCVS / UCLS   | Conteúdos molares por componente no separador |
| 17                | ETS           | Energia interna total do separador            |
| 18–25             | UCLC          | Conteúdos molares por componente no stripper  |
| 26                | ETC           | Energia interna total do stripper             |
| 27–34             | UCVV          | Conteúdos molares por componente no compressor|
| 35                | ETV           | Energia interna total do compressor           |
| 36                | TWR           | Temperatura da água de resfriamento do reator |
| 37                | TWS           | Temperatura da água de resfriamento do sep.   |
| 38–49             | VPOS[0..11]   | Posições das 12 válvulas                      |

---

## Sobre Temperaturas de Água de Resfriamento (TWR, TWS)

TWR (`yy[36]`) e TWS (`yy[37]`) existem no vetor de estados por compatibilidade estrutural com TEINIT. No modelo de referência Downs-Vogel (FORTRAN TEFUNC), `YP(37)` e `YP(38)` nunca são atribuídos — suas derivadas são implicitamente zero. Essas temperaturas são lidas do estado inicial e permanecem constantes durante toda a integração; são moduladas apenas pelos canais de distúrbio IDV[3] e IDV[4] como perturbações externas. No modelo Rust, isso é explicitado como `yp[36] = 0.0; yp[37] = 0.0`.

---

## Sobre Coeficientes de Entalpia e Escala

O modelo usa dois conjuntos de coeficientes de entalpia:

- **AG[i]**: coeficientes de entalpia de vapor (fase gasosa), escala `~1e-3`
- **AH[i]**: coeficientes de entalpia líquida, escala `~1e-6` (ex.: `AH[3] = 0.960e-6`)

Essa assimetria de escala é intencional e deve ser considerada ao copiar valores de energia do FORTRAN TEINIT para o estado inicial Rust. Os valores de energia interna de unidades predominantemente líquidas (ETS, ETC, ETV) precisam ser convertidos de acordo com a escala de AH usada no Rust. ETR é dominado pela fase vapor e usa AG, portanto não requer conversão.

---

## Premissas para o Desacoplamento dos Controladores da Planta

Estas premissas governam a refatoração descrita no Experimento 11 e qualquer experimento subsequente que altere a camada de controle.

**1. Ordem do loop de simulação**

A ordem correta do tick de simulação é:

```
plant.step(dt)         ← integra o modelo com os mv atuais
ramp_logic()           ← atualiza feed valves durante cold start
bank.step(xmeas, xmv)  ← controladores calculam e escrevem novos mv
```

O mv calculado no passo k é aplicado no passo k+1. Isso introduz um atraso de um passo — semântica de controle discreta padrão. Inverter a ordem (`bank.step → plant.step`) eliminaria esse atraso mas produziria CSV numericamente diferente do baseline do Exp 10, invalidando a comparação direta. Qualquer alteração dessa ordem constitui uma mudança de comportamento e exige novo baseline de referência.

**2. Escrita em XMV e conflito de comando**

Cada `Controller` escreve diretamente numa posição do vetor `xmv`. O `ControllerBank` aplica cada controlador sequencialmente sem arbitragem. Se dois controladores escreverem no mesmo índice, o último na sequência vence — não há lógica de seleção/override. Nos experimentos atuais cada MV tem exatamente um controlador, portanto o conflito não ocorre. Lógica de seleção (e.g. split-range, override selector) é responsabilidade de experimentos futuros que introduzam múltiplos controladores por MV.

**3. Responsabilidade da trait `Controller`**

A interface `fn step(&mut self, xmeas: &[f64], xmv: &mut [f64])` combina observação, cálculo e escrita numa única chamada. Isso é suficiente para composição sequencial no `ControllerBank` e para reprodução do baseline. Separar decisão de controle de aplicação do comando (e.g. `compute() → Command`, `apply(Command, xmv)`) só se torna necessário quando houver lógica supervisória que precise interceptar ou modificar o comando antes de enviá-lo ao atuador — o que está fora do escopo do Exp 11.

---

## Sobre o Estado Inicial e Cold Start

O arquivo de estado inicial (`te_mode1_initial_state.toml`) não é um steady-state do modelo Rust — é uma transcrição dos valores TEINIT do FORTRAN. O estado inicial serve como condição de partida com holdups não-zero em todos os vasos, garantindo que as equações do modelo (frações molares, temperaturas) não encontrem denominadores zero.

A estratégia de entrada em operação é o **cold start**: as válvulas de feed (`mv[0..4]`) são fechadas em `t=0` e rampadas linearmente de 0% até os valores nominais do TOML em `ramp_duration` horas de tempo simulado. Controladores P de pressão do reator (`mv[5]`), nível do separador (`mv[6]`) e nível do stripper (`mv[7]`) ficam ativos desde o início. Distúrbios IDV são desligados durante a rampa e ativados automaticamente ao final. Isso permite que a planta encontre seu próprio ponto de operação gradualmente, sem exigir que o estado inicial satisfaça `ẋ = 0`.

---

