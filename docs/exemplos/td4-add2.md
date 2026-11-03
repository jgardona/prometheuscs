---
exemplo: td4-add2.txt
titulo: TD4 Add 2
categoria: Sequential Logic > TD4 CPU
verificacoes:
  - t=0.0097 elemento=58:LEDElm grandeza=i esperado=0.003322 tol=0.01
  - t=0.0097 elemento=60:LEDElm grandeza=i esperado=0.003322 tol=0.01
  - t=0.0097 elemento=56:LEDElm grandeza=i esperado=0 abs=0.00001
  - t=0.0097 elemento=46:LEDElm grandeza=i esperado=0.003322 tol=0.01 alterna=78
  - t=0.0097 elemento=56:LEDElm grandeza=i esperado=0 abs=0.00001 alterna=78
  - t=0.0097 elemento=60:LEDElm grandeza=i esperado=0.003322 tol=0.01 alterna=78
  - t=0.0097 elemento=58:LEDElm grandeza=i esperado=0.003322 tol=0.01 alterna=81
  - t=0.0097 elemento=60:LEDElm grandeza=i esperado=0 abs=0.00001 alterna=81
---

# CPU TD4: entrada mais 2

<!-- navegacao:inicio -->
[Índice](README.md) · [Anterior: TD4 Up-Down Counter](td4-ctr-up-dn.md)
<!-- navegacao:fim -->

## Objetivo

Mostrar a TD4 lendo a porta de **entrada**: o programa lê as quatro chaves, soma $2$ e mostra o resultado nos LEDs, continuamente.

## O circuito

<!-- esquema:inicio -->
![Esquema do circuito](img/td4-add2.svg)
<!-- esquema:fim -->

A mesma CPU do [TD4 LED Strobe](td4.md); só o programa da ROM muda. A porta de entrada são as quatro chaves *In*, de cima para baixo os bits 3 a 0 (elementos 78 a 81). O exemplo abre com $In = 0001$.

<!-- componentes:inicio -->
| N.º | Tipo | Elemento | Valores |
|----:|------|----------|---------|
| 0 | `Counter2Elm` | Counter | High Logic Voltage = 5; Bit Order = Bus; # of Bits = 4; Modulus = 0 |
| 1 | `LabeledNodeElm` | Labeled Node | Text = /LOAD0; Internal Node = não; Rotate Text When Vertical = não |
| 2 | `LabeledNodeElm` | Labeled Node | Text = /RESET; Internal Node = não; Rotate Text When Vertical = não |
| 4 | `LabeledNodeElm` | Labeled Node | Text = CLOCK; Internal Node = não; Rotate Text When Vertical = não |
| 8 | `LabeledNodeElm` | Labeled Node | Text = /LOAD1; Internal Node = não; Rotate Text When Vertical = não |
| 9 | `Counter2Elm` | Counter | High Logic Voltage = 5; Bit Order = Bus; # of Bits = 4; Modulus = 0 |
| 10 | `Counter2Elm` | Counter | High Logic Voltage = 5; Bit Order = Bus; # of Bits = 4; Modulus = 0 |
| 11 | `LabeledNodeElm` | Labeled Node | Text = /LOAD2; Internal Node = não; Rotate Text When Vertical = não |
| 14 | `Counter2Elm` | Counter | High Logic Voltage = 5; Bit Order = Bus; # of Bits = 4; Modulus = 0 |
| 15 | `LabeledNodeElm` | Labeled Node | Text = /LOAD3; Internal Node = não; Rotate Text When Vertical = não |
| 31 | `MultiplexerElm` | Multiplexer | High Logic Voltage = 5; # of Select Bits = 2; Inverted Output = não; Strobe Pin = não; Input Mode = Bus Input/Output; Bus Select = não; Data Bus Width = 4 |
| 33 | `RoutedWireElm` | wire | Show Current = não; Show Voltage = não; Show Bus Value = sim; Show Bus Value (Hex) = não |
| 34 | `LabeledNodeElm` | Labeled Node | Text = SEL_A; Internal Node = não; Rotate Text When Vertical = não |
| 35 | `LabeledNodeElm` | Labeled Node | Text = SEL_B; Internal Node = não; Rotate Text When Vertical = não |
| 36 | `FullAdderElm` | Adder | High Logic Voltage = 5; Bit Order = Bus; # of Bits = 4 |
| 38 | `RoutedWireElm` | wire | Show Current = não; Show Voltage = não; Show Bus Value = sim; Show Bus Value (Hex) = não |
| 39 | `BusLogicInputElm` | bus input (4) | Bus Width = 4; Value = 0; High Voltage = 5; Low Voltage = 0 |
| 40 | `BusSplitterElm` | Bus Splitter | High Logic Voltage = 5; # of Bits = 4 |
| 41 | `BusSplitterElm` | Bus Splitter | High Logic Voltage = 5; # of Bits = 4 |
| 42 | `ResistorElm` | resistor | Resistance (ohms) = 1k |
| 43 | `ResistorElm` | resistor | Resistance (ohms) = 1k |
| 44 | `ResistorElm` | resistor | Resistance (ohms) = 1k |
| 45 | `ResistorElm` | resistor | Resistance (ohms) = 1k |
| 46 | `LEDElm` | LED | Red Value (0-1) = 1; Green Value (0-1) = 0; Blue Value (0-1) = 0; Max Brightness Current (A) = 10m; Model = default-led |
| 47 | `GroundElm` | ground | Symbol = Signal |
| 48 | `GroundElm` | ground | Symbol = Signal |
| 49 | `GroundElm` | ground | Symbol = Signal |
| 50 | `GroundElm` | ground | Symbol = Signal |
| 51 | `GroundElm` | ground | Symbol = Signal |
| 52 | `GroundElm` | ground | Symbol = Signal |
| 53 | `GroundElm` | ground | Symbol = Signal |
| 54 | `GroundElm` | ground | Symbol = Signal |
| 55 | `GroundElm` | ground | Symbol = Signal |
| 56 | `LEDElm` | LED | Red Value (0-1) = 1; Green Value (0-1) = 0; Blue Value (0-1) = 0; Max Brightness Current (A) = 10m; Model = default-led |
| 57 | `GroundElm` | ground | Symbol = Signal |
| 58 | `LEDElm` | LED | Red Value (0-1) = 1; Green Value (0-1) = 0; Blue Value (0-1) = 0; Max Brightness Current (A) = 10m; Model = default-led |
| 59 | `GroundElm` | ground | Symbol = Signal |
| 60 | `LEDElm` | LED | Red Value (0-1) = 1; Green Value (0-1) = 0; Blue Value (0-1) = 0; Max Brightness Current (A) = 10m; Model = default-led |
| 61 | `RoutedWireElm` | wire | Show Current = não; Show Voltage = não; Show Bus Value = não; Show Bus Value (Hex) = não |
| 62 | `RoutedWireElm` | wire | Show Current = não; Show Voltage = não; Show Bus Value = não; Show Bus Value (Hex) = não |
| 63 | `RoutedWireElm` | wire | Show Current = não; Show Voltage = não; Show Bus Value = não; Show Bus Value (Hex) = não |
| 64 | `RoutedWireElm` | wire | Show Current = não; Show Voltage = não; Show Bus Value = não; Show Bus Value (Hex) = não |
| 65 | `BusSplitterElm` | Bus Splitter | High Logic Voltage = 5; # of Bits = 8 |
| 66 | `BusSplitterElm` | Bus Splitter | High Logic Voltage = 5; # of Bits = 4 |
| 67 | `RoutedWireElm` | wire | Show Current = não; Show Voltage = não; Show Bus Value = sim; Show Bus Value (Hex) = não |
| 68 | `CustomCompositeElm` | subcircuit (decoder) | Model Name = decoder; High Logic Voltage (0=default) = 0 |
| 69 | `RoutedWireElm` | wire | Show Current = não; Show Voltage = não; Show Bus Value = não; Show Bus Value (Hex) = não |
| 70 | `RoutedWireElm` | wire | Show Current = não; Show Voltage = não; Show Bus Value = não; Show Bus Value (Hex) = não |
| 71 | `RoutedWireElm` | wire | Show Current = não; Show Voltage = não; Show Bus Value = não; Show Bus Value (Hex) = não |
| 72 | `LabeledNodeElm` | Labeled Node | Text = /LOAD0; Internal Node = não; Rotate Text When Vertical = não |
| 73 | `LabeledNodeElm` | Labeled Node | Text = /LOAD1; Internal Node = não; Rotate Text When Vertical = não |
| 74 | `LabeledNodeElm` | Labeled Node | Text = /LOAD2; Internal Node = não; Rotate Text When Vertical = não |
| 75 | `LabeledNodeElm` | Labeled Node | Text = /LOAD3; Internal Node = não; Rotate Text When Vertical = não |
| 76 | `LabeledNodeElm` | Labeled Node | Text = SEL_A; Internal Node = não; Rotate Text When Vertical = não |
| 77 | `RoutedWireElm` | wire | Show Current = não; Show Voltage = não; Show Bus Value = não; Show Bus Value (Hex) = não |
| 78 | `LogicInputElm` | logic input | Momentary Switch = não; High Logic Voltage = 5; Low Voltage = 0; Numeric = não; Ternary = não; Keyboard Shortcut =  |
| 79 | `LogicInputElm` | logic input | Momentary Switch = não; High Logic Voltage = 5; Low Voltage = 0; Numeric = não; Ternary = não; Keyboard Shortcut =  |
| 80 | `LogicInputElm` | logic input | Momentary Switch = não; High Logic Voltage = 5; Low Voltage = 0; Numeric = não; Ternary = não; Keyboard Shortcut =  |
| 81 | `LogicInputElm` | logic input | Momentary Switch = não; High Logic Voltage = 5; Low Voltage = 0; Numeric = não; Ternary = não; Keyboard Shortcut =  |
| 82 | `RoutedWireElm` | wire | Show Current = não; Show Voltage = não; Show Bus Value = não; Show Bus Value (Hex) = não |
| 83 | `DFlipFlopElm` | D flip-flop | High Logic Voltage = 5; Reset Pin = sim; Set Pin = não; Invert Set/Reset = sim |
| 85 | `LabeledNodeElm` | Labeled Node | Text = CLOCK; Internal Node = não; Rotate Text When Vertical = não |
| 86 | `LabeledNodeElm` | Labeled Node | Text = /RESET; Internal Node = não; Rotate Text When Vertical = não |
| 87 | `LogicInputElm` | logic input | Momentary Switch = não; High Logic Voltage = 5; Low Voltage = 0; Numeric = não; Ternary = não; Keyboard Shortcut =  |
| 88 | `RailElm` | square wave gen | Max Voltage = 2.5; Waveform = Square Wave; DC Offset (V) = 2.5; Internal Resistance (ohms) = 0; Show Voltage = não; Specify As = Frequency/Duty Cycle; Frequency (Hz) = 1k; Phase Offset (degrees) = 0; Duty Cycle = 50; Rise/Fall Time (s) = 0 |
| 90 | `RoutedWireElm` | wire | Show Current = não; Show Voltage = não; Show Bus Value = não; Show Bus Value (Hex) = não |
| 93 | `RailElm` | voltage source | Voltage = 5; Waveform = D/C; DC Offset (V) = 0; Internal Resistance (ohms) = 0 |
| 94 | `RailElm` | voltage source | Voltage = 5; Waveform = D/C; DC Offset (V) = 0; Internal Resistance (ohms) = 0 |
| 97 | `TextElm` | text | Text = A; Size = 24; Draw Bar On Top = não; Color = #c0c0c0 |
| 98 | `TextElm` | text | Text = B; Size = 24; Draw Bar On Top = não; Color = #c0c0c0 |
| 99 | `TextElm` | text | Text = PC; Size = 24; Draw Bar On Top = não; Color = #c0c0c0 |
| 100 | `TextElm` | text | Text = Out; Size = 24; Draw Bar On Top = não; Color = #c0c0c0 |
| 101 | `TextElm` | text | Text = Command Decoder; Size = 24; Draw Bar On Top = não; Color = #c0c0c0 |
| 102 | `TextElm` | text | Text = In; Size = 24; Draw Bar On Top = não; Color = #c0c0c0 |
| 103 | `RoutedWireElm` | wire | Show Current = não; Show Voltage = não; Show Bus Value = não; Show Bus Value (Hex) = sim |
| 104 | `InstructionDisplayElm` | instruction display | Bus Width = 8; Threshold Voltage = 2.5; Lookup Table = 0-0xF=ADD A, {a} 0x10-0x1F=MOV A, B 0x20-0x2F=IN A 0x30-0x3F |
| 105 | `TextElm` | text | Text = Current Instruction; Size = 24; Draw Bar On Top = não; Color = #c0c0c0 |
| 107 | `ROMElm` | ROM | High Logic Voltage = 5; Bit Order = Bus; # of Address Bits = 4; # of Data Bits = 8; Contents = 0: 60 52 90 F0 ; Hex Display = sim; Restore Contents on Reset = não |
| 108 | `LogicInputElm` | logic input | Momentary Switch = não; High Logic Voltage = 5; Low Voltage = 0; Numeric = não; Ternary = não; Keyboard Shortcut =  |
<!-- componentes:fim -->

## Teoria

| Endereço | Código | Instrução |
|------|------|------|
| 0 | `60` | `IN B` |
| 1 | `52` | `ADD B, 2` |
| 2 | `90` | `OUT B` |
| 3 | `F0` | `JMP 0` |

A saída é $(In + 2) \bmod 16$, atualizada a cada volta do laço de quatro instruções ($4\ \text{ms}$). Mudando as chaves, o novo valor aparece na saída na volta seguinte.

## Resultados esperados

O simulador dá, com $In = 1$, a saída $3$ ($0011$); com $In = 9$ ($1001$, a chave do bit 3 ligada), a saída $11$ ($1011$); com $In = 0$, a saída $2$ ($0010$).

## O que observar

Mude as chaves e veja os LEDs acompanharem com até uma volta do laço de atraso. Experimente $In = 14$ ou $15$: a soma passa de $15$ e o resultado dá a volta ($0$ ou $1$).

## Limites do modelo

- Os mesmos do [TD4 LED Strobe](td4.md).

## Referências

Iku Watanabe, *CPUの創りかた*, Mynavi, 2003.

O circuito deste exemplo faz parte da coleção do simulador original de Paul Falstad e Iain Sharp (CircuitJS1); a explicação é deste projeto.
