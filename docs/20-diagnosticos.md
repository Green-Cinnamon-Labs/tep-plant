# Diagnósticos (`src/diagnostics/`) — o que cada parte faz e por quê

Este documento reúne o que antes estava em comentários dentro de `src/diagnostics/shutdown_detector.rs` (o código ficou só com o código, no mesmo padrão de `docs/12-reator.md`). Os diagnósticos agregam mais de uma unidade ao mesmo tempo; hoje só existe `ShutdownDetector`. Os testes ficam em `tests/diagnostics/shutdown_detector.rs`.

## Origem

`ShutdownDetector` é a detecção de shutdown do bloco 36 do `teprob.f`, migrada do antigo `dynamics/measured.rs` (issue 10). É o único pedaço de `Measured` sem dono natural: agrega Reactor, Separator e Stripper ao mesmo tempo, então não faz sentido virar tarefa de nenhuma unidade. Também não é um acumulador físico, por isso não mora em `units/`, e sim num módulo próprio, `diagnostics`.

## O sinal

Publica `status.shutdown_detected`: 1,0 se qualquer limite de segurança for violado, 0,0 caso contrário. O prefixo é `status.`, não `xmeas.`, porque não é uma das 41 XMEAS do TEP: é um diagnóstico à parte, equivalente ao antigo `isd_active` do gRPC. É só diagnóstico: o sinal é exposto pelo sensor `status.shutdown_detected` e nada na planta reage a ele — a simulação continua rodando mesmo com o sinal em 1. O interlock de verdade é a #70.

## Limites

Os mesmos do bloco 36 do `teprob.f` (linhas 991 a 998):

| Condição | Limite |
|---|---|
| Pressão do reator | > 3000 kPa |
| Volume de líquido do reator | < 2 m³ ou > 24 m³ |
| Temperatura do reator | > 175 °C |
| Volume de líquido do separador | < 1 m³ ou > 12 m³ |
| Volume de líquido do stripper | < 1 m³ ou > 8 m³ |

Os volumes são lidos brutos (`reactor.liquid_volume`, `separator.liquid_volume`, `stripper.liquid_volume`), em pé cúbico, a unidade interna do `teprob.f`, e convertidos para m³ dividindo por 35,3145 (ft³ por m³). São brutos, e não as XMEAS de nível, porque os limites são absolutos de segurança: as XMEAS 8, 12 e 15 são relativas ao zero e à faixa de calibração dos instrumentos (por exemplo, `XMEAS(8) = (VLR − 84,6)/666,7 × 100`). A pressão, ao contrário, é a XMEAS já convertida (`xmeas.reactor.pressure`, em kPa): o limite de 3000 é expresso nessa unidade no original, não em mmHg bruto.
