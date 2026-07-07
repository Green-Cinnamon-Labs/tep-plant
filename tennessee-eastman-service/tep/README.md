# Tennessee Eastman Process (TEP) – Referência do Diagrama e Variáveis

Este documento descreve **formalmente** os elementos do diagrama do Tennessee Eastman Process (TEP), servindo como referência canônica para mapeamento de estados, medições e atuadores no código (`te_core` e `te_service`).

O objetivo **não** é controle nem modelagem detalhada aqui, mas **identificação inequívoca do que é o quê**.

---

## 1. Unidades de Processo

O processo contém cinco unidades principais:

1. **Reactor**
   - Reator multifásico, exotérmico
   - Contém agitador e serpentina de resfriamento
2. **Condenser**
   - Condensa efluente do reator
   - Usa água de resfriamento - CWS (Cooling Water Supply) / CWR (Cooling Water Return)
3. **Vapor–Liquid Separator**
   - Separa fase vapor e líquida após condensação
4. **Recycle Compressor**
   - Recircula gases não condensados ao reator
   - Possui válvula de recycle e proteção contra surge
5. **Stripper**
   - Remove reagentes remanescentes do produto
   - Utiliza vapor (steam) e condensado

---

## 2. Correntes de Processo (Streams)

| Nº | Nome                | Descrição |
|----|---------------------|-----------|
| 1  | A feed              | Alimentação de A |
| 2  | D feed              | Alimentação de D |
| 3  | E feed              | Alimentação de E |
| 4  | A + C feed          | Alimentação combinada A/C |
| 5  | Recycle gas         | Gás reciclado |
| 6  | Reactor feed        | Entrada total do reator |
| 7  | Reactor effluent    | Saída do reator |
| 8  | Compressor outlet   | Saída do compressor |
| 9  | Purge               | Purga de gás |
| 10 | Separator liquid    | Líquido do separador |
| 11 | Product             | Produto final |
| 12 | Reactor cooling     | Água de resfriamento do reator |
| 13 | Condenser cooling   | Água de resfriamento do condensador |

---

## 3. Variáveis Manipuladas (XMV – Atuadores)

Total: **12 variáveis manipuladas**, todas normalizadas em 0–100%.

| XMV | Descrição |
|-----|-----------|
| 1 | Vazão D feed (stream 2) |
| 2 | Vazão E feed (stream 3) |
| 3 | Vazão A feed (stream 1) |
| 4 | Vazão A+C feed (stream 4) |
| 5 | Válvula de recycle do compressor |
| 6 | Válvula de purge (stream 9) |
| 7 | Vazão de líquido do separador (stream 10) |
| 8 | Vazão de produto do stripper (stream 11) |
| 9 | Válvula de vapor do stripper |
| 10 | Água de resfriamento do reator |
| 11 | Água de resfriamento do condensador |
| 12 | Velocidade do agitador |

---

## 4. Medições Contínuas (XMEAS – Sensores)

As XMEAS representam **variáveis medidas do processo** (equivalentes funcionais a FI, TI, PI, LI),
calculadas a partir do estado interno da planta.  
Elas **não são dispositivos físicos**, mas os valores que esses dispositivos reportariam.

Abaixo estão **todas as medições contínuas do TEP**, o que medem e **qual XMV está fisicamente associado** (quando aplicável).

---

### 4.1 Vazões (Flow – FI)

| XMEAS | Mede | Stream | XMV associada |
|------|------|--------|--------------|
| 1 | Vazão A feed | 1 | XMV(3) |
| 2 | Vazão D feed | 2 | XMV(1) |
| 3 | Vazão E feed | 3 | XMV(2) |
| 4 | Vazão A+C feed | 4 | XMV(4) |
| 5 | Vazão de recycle | 8 | XMV(5) |
| 6 | Vazão total para o reator | 6 | (derivada dos feeds) |
| 10 | Vazão de purge | 9 | XMV(6) |
| 14 | Vazão líquida do separador | 10 | XMV(7) |
| 17 | Vazão de produto | 11 | XMV(8) |

---

### 4.2 Pressões (Pressure – PI)

| XMEAS | Mede | Unidade | XMV associada |
|------|------|--------|--------------|
| 7 | Pressão do reator | Reactor | — |
| 13 | Pressão do separador | Separator | — |
| 16 | Pressão do stripper | Stripper | — |

---

### 4.3 Níveis (Level – LI)

| XMEAS | Mede | Unidade | XMV associada |
|------|------|--------|--------------|
| 8 | Nível do reator | Reactor | XMV(7), XMV(8) |
| 12 | Nível do separador | Separator | XMV(7) |
| 15 | Nível do stripper | Stripper | XMV(8) |

---

### 4.4 Temperaturas de Processo (Temperature – TI)

| XMEAS | Mede | Unidade | XMV associada |
|------|------|--------|--------------|
| 9 | Temperatura do reator | Reactor | XMV(10), XMV(12) |
| 11 | Temperatura do separador | Separator | XMV(11) |
| 18 | Temperatura do stripper | Stripper | XMV(9) |

---

### 4.5 Utilidades Térmicas (Cooling Water)

| XMEAS | Mede | Local | XMV associada |
|------|------|------|--------------|
| 21 | Temperatura de saída da água do reator (CWR) | Reactor | XMV(10) |
| 22 | Temperatura de saída da água do condensador (CWR) | Condenser | XMV(11) |

---

### 4.6 Energia / Trabalho

| XMEAS | Mede | Unidade | XMV associada |
|------|------|--------|--------------|
| 20 | Trabalho do compressor | Compressor | XMV(5) |

---

### Observações Importantes

- Nem toda XMEAS tem uma XMV direta (ex.: pressões e níveis são **resultantes do balanço**).
- XMV controla **válvulas / vazões / utilidades**, XMEAS observa o efeito físico.
- Este mapeamento é a base para:
  - logging interpretável
  - supervisão
  - futura instrumentação virtual (tags)

---

## 5. Analisadores de Composição (XMEAS Amostrados)

Analisadores químicos com:
- Frequência de amostragem
- Tempo morto (dead time)

### Pontos de análise:
1. **Feed do reator (stream 6)**
2. **Purge (stream 9)**
3. **Produto (stream 11)**

### Componentes analisados:
- A, B, C, D, E, F, G, H (dependendo do ponto)

---

## 6. Distúrbios de Processo (DV / IDV)

Variáveis externas que afetam a planta, ativadas por flags.

### Exemplos:
- Mudança de composição dos feeds
- Variação de temperatura de feeds
- Variação de temperatura da água de resfriamento
- Perda de feed
- Alterações de cinética
- Válvulas travando
- Ruído aleatório

---

## 7. Correspondência com a Arquitetura de Código

### `te_core`
Responsável por:
- Estados contínuos da planta
- Dinâmica (EDOs)
- Cálculo de XMEAS
- Aplicação de XMV e DV

Não contém:
- I/O
- Logs
- Tempo real
- Threads

### `te_service`
Responsável por:
- Loop de simulação
- Avanço de tempo lógico
- Logging / exportação (println, CSV, etc.)
- Interface externa futura

---

## 8. Próximo Passo

Definir **formalmente o mapeamento**:
- `state[i]` → significado físico
- `output[j]` → XMEAS
- `mv[k]` → XMV
- `dv[m]` → distúrbios

Isso será feito via **metadata explícita** (enum / tabela / struct).

---

Documento base para rastreabilidade entre:
**artigo original ↔ diagrama ↔ código Rust**.



## Sobre a modelagem do projeto

```bash
[ Service Loop ]
      |
      v
  PlantBus
   ├─ time
   ├─ inputs  ──────────────────┐
   ├─ outputs ◄─────────────┐   │
   └─ plant (core::Plant)   │   │
                            │   │
                        core::Plant
                        ├─ state
                        ├─ params
                        ├─ dynamics
                        └─ integrator
```