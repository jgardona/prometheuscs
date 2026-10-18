---
exemplo: relayand.txt
titulo: Relay AND
categoria: Other Passive Circuits > Relays
verificacoes:
  - t=0.1 elemento=17:LogicOutputElm grandeza=vd esperado=0 abs=0.01
  - t=0.1 elemento=17:LogicOutputElm grandeza=vd esperado=0 abs=0.01 alterna=15,16
  - t=0.1 elemento=17:LogicOutputElm grandeza=vd esperado=0 abs=0.01 alterna=14
  - t=0.1 elemento=17:LogicOutputElm grandeza=vd esperado=0 abs=0.01 alterna=14,15
  - t=0.1 elemento=17:LogicOutputElm grandeza=vd esperado=4.99251 tol=0.01 alterna=14,15,16
  - t=0.004 elemento=17:LogicOutputElm grandeza=vd esperado=0 abs=0.01 alterna=14,15,16
  - t=0.012 elemento=17:LogicOutputElm grandeza=vd esperado=4.99251 tol=0.01 alterna=14,15,16
---

# Porta E com relés

<!-- navegacao:inicio -->
[Índice](README.md) · [Anterior: Relay](relay.md) · [Próximo: Relay OR](relayor.md)
<!-- navegacao:fim -->

## Objetivo

Mostrar uma porta lógica E (AND) de três entradas feita só com relés: a saída só vai a 1 quando as três entradas estão em 1.

## O circuito

<!-- esquema:inicio -->
![Esquema do circuito](img/relayand.svg)
<!-- esquema:fim -->

Três relés têm seus contatos normalmente abertos ligados **em série**: a fonte de $5\ \text{V}$ entra no polo do primeiro; o contato do primeiro leva ao polo do segundo, e assim por diante até a saída, que tem um resistor de $100\ \Omega$ para a terra e um indicador de nível lógico. Cada relé é comandado por uma entrada lógica (os três botões à esquerda, inicialmente em 0).

<!-- componentes:inicio -->
| N.º | Tipo | Elemento | Valores |
|----:|------|----------|---------|
| 0 | `RelayElm` | relay (old-relay-2) | Model = old-relay-2 |
| 1 | `RelayElm` | relay (old-relay-2) | Model = old-relay-2 |
| 2 | `RelayElm` | relay (old-relay-2) | Model = old-relay-2 |
| 3 | `RailElm` | voltage source | Voltage = 5; Waveform = D/C; DC Offset (V) = 0; Internal Resistance (ohms) = 0 |
| 8 | `GroundElm` | ground | Symbol = Earth |
| 9 | `GroundElm` | ground | Symbol = Earth |
| 10 | `GroundElm` | ground | Symbol = Earth |
| 14 | `LogicInputElm` | logic input | Momentary Switch = não; High Logic Voltage = 5; Low Voltage = 0; Numeric = não; Ternary = não; Keyboard Shortcut =  |
| 15 | `LogicInputElm` | logic input | Momentary Switch = não; High Logic Voltage = 5; Low Voltage = 0; Numeric = não; Ternary = não; Keyboard Shortcut =  |
| 16 | `LogicInputElm` | logic input | Momentary Switch = não; High Logic Voltage = 5; Low Voltage = 0; Numeric = não; Ternary = não; Keyboard Shortcut =  |
| 17 | `LogicOutputElm` | logic output | Threshold = 2.5; Current Required = não; Numeric = não; Ternary = não |
| 18 | `ResistorElm` | resistor | Resistance (ohms) = 100 |
| 19 | `GroundElm` | ground | Symbol = Earth |
<!-- componentes:fim -->

## Teoria

Um relé é uma chave operada por um eletroímã. Seu modelo no simulador tem duas partes:

- a **bobina**, uma indutância $L$ em série com uma resistência $R_b$; com a corrente $i_b$ crescendo como $i_b(t) = \dfrac{V}{R_b}\left(1 - e^{-t/\tau_b}\right)$, $\tau_b = L/R_b$, o relé **opera** quando $i_b \ge I_{on}$ e **solta** quando $i_b \le I_{off}$;
- os **contatos**, com resistência $R_{on} = 0{,}05\ \Omega$ quando fechados e $R_{off} = 1\ \text{M}\Omega$ quando abertos (os terminais abertos são ligados à terra por essa resistência de $1\ \text{M}\Omega$), e um tempo de $5\ \text{ms}$ para a armadura mudar de posição.

Os relés destes exemplos têm um polo com dois contatos: **normalmente aberto** (liga o polo quando a bobina está energizada) e **normalmente fechado**. Com isso se constroem portas lógicas: contatos em **série** fazem E, contatos em **paralelo** fazem OU, e a ligação cruzada de dois relés faz OU exclusivo. Os níveis lógicos dos exemplos são $5\ \text{V}$ (nível 1) e $0\ \text{V}$ (nível 0); uma entrada em 1 aplica $5\ \text{V}$ à bobina e a energiza.

Aqui as bobinas têm $L = 0{,}1\ \text{H}$ e $R_b = 50\ \Omega$, logo $\tau_b = 2\ \text{ms}$ e a corrente final é $5/50 = 100\ \text{mA}$, bem acima de $I_{on} = I_{off} = 20\ \text{mA}$. O relé opera depois de $t = -\tau_b \ln(1 - I_{on}R_b/V) = 2\ \text{ms}\cdot\ln(1/0{,}8) = 0{,}45\ \text{ms}$, mais os $5\ \text{ms}$ da armadura: a porta leva cerca de $5{,}5\ \text{ms}$ para responder. Para soltar, a corrente cai de $100\ \text{mA}$ a $20\ \text{mA}$ em $\tau_b\ln 5 = 3{,}2\ \text{ms}$, mais $5\ \text{ms}$: cerca de $8{,}2\ \text{ms}$.

## Resultados esperados

Para a saída ser $5\ \text{V}$ é preciso que os três contatos estejam fechados:

| $A$ | $B$ | $C$ | $Y$ |
|:--:|:--:|:--:|:--:|
| 0 | 0 | 0 | 0 |
| 0 | 0 | 1 | 0 |
| 0 | 1 | 0 | 0 |
| 0 | 1 | 1 | 0 |
| 1 | 0 | 0 | 0 |
| 1 | 0 | 1 | 0 |
| 1 | 1 | 0 | 0 |
| 1 | 1 | 1 | 1 |

Só a última linha vale 1: $Y = A\cdot B\cdot C$. Os níveis de saída dependem de uma resistência de carga de $100\ \Omega$ ligada à terra, em série com $R_{on} = 0{,}05\ \Omega$ por contato fechado: com $k$ contatos fechados em série, $V_{saida} = 5\cdot\dfrac{100}{100 + 0{,}05\,k}$, ou seja, um pouco abaixo de $5\ \text{V}$. Com $k = 3$, $V_{saida} = 5\cdot 100/100{,}15 = 4{,}9925\ \text{V}$.

As verificações conferem cinco das oito combinações (a saída é $0$ em quatro delas e $4{,}9925\ \text{V}$ quando as três entradas estão em 1) e o atraso: com as três entradas ligadas em $t = 0$, a saída ainda é $0$ em $4\ \text{ms}$ e já é $4{,}9925\ \text{V}$ em $12\ \text{ms}$, em acordo com os $5{,}5\ \text{ms}$ estimados.

## O que observar

- Clique nas entradas: só com as três em 1 a saída acende.
- Com duas entradas em 1, siga com o ponteiro o caminho da corrente: ela passa pelos dois primeiros contatos e para no terceiro, aberto.
- Observe o atraso de alguns milissegundos entre mudar uma entrada e a saída responder.

## Limites do modelo

Relés ideais no sentido do modelo: resistência de contato fixa, sem arco elétrico, sem ricochete dos contatos e sem desgaste. Os tempos de resposta calculados (cerca de $5{,}5\ \text{ms}$ para operar e $8{,}2\ \text{ms}$ para soltar) vêm só da bobina e do tempo da armadura; num relé real dependem também da mola e da carga mecânica. As verificações fixam as entradas lógicas antes da simulação (ou em instantes dados).

## Referências

- P. Horowitz e W. Hill, *The Art of Electronics*, Cambridge University Press.
- C. K. Alexander e M. N. O. Sadiku, *Fundamentos de Circuitos Elétricos*, AMGH.

O circuito deste exemplo faz parte da coleção do simulador original de Paul Falstad e Iain Sharp (CircuitJS1); a explicação é deste projeto.
