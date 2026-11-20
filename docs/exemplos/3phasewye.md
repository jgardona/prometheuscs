---
exemplo: 3phasewye.txt
titulo: 3-Phase Wye-Wye Balanced Load
categoria: Other Passive Circuits
verificacoes:
  - t=0.0667 de=0.05 elemento=3:ResistorElm grandeza=vd fn=rms esperado=120 tol=0.01
  - t=0.0667 de=0.05 elemento=5:ResistorElm grandeza=vd fn=rms esperado=120 tol=0.01
  - t=0.0667 de=0.05 elemento=6:ResistorElm grandeza=vd fn=rms esperado=120 tol=0.01
  - t=0.0667 de=0.05 elemento=13:SwitchElm grandeza=i fn=rms esperado=0 abs=0.001
  - t=0.0667 de=0.05 elemento=13:SwitchElm grandeza=i fn=rms esperado=0 abs=0.001 alterna=13
---

# Sistema trifásico estrela-estrela equilibrado

<!-- navegacao:inicio -->
[Índice](README.md) · [Anterior: Phase-Sequence Network](phaseseq.md)
<!-- navegacao:fim -->

## Objetivo

Mostrar um sistema trifásico equilibrado, com três fontes defasadas de $120^\circ$ ligadas em estrela a três cargas iguais, e que num sistema equilibrado a corrente do fio neutro é nula (por isso o neutro pode até ser retirado).

## O circuito

<!-- esquema:inicio -->
![Esquema do circuito](img/3phasewye.svg)
<!-- esquema:fim -->

Três fontes senoidais de $169{,}7\ \text{V}$ de pico ($120\ \text{V}$ eficazes), $60\ \text{Hz}$, com fases de $0^\circ$ (fase A), $120^\circ$ (fase B) e $240^\circ$ (fase C), têm um terminal comum (o neutro da fonte). Cada uma alimenta um resistor de $1\ \text{k}\Omega$; os outros terminais dos resistores se ligam no neutro da carga, que é ligado ao neutro da fonte por um fio com uma chave (o "retorno neutro"). Dois osciloscópios mostram as tensões e as correntes das três fases.

<!-- componentes:inicio -->
| N.º | Tipo | Elemento | Valores |
|----:|------|----------|---------|
| 0 | `VoltageElm` | A/C source | Max Voltage = 169.7; Waveform = A/C; DC Offset (V) = 0; Internal Resistance (ohms) = 0; Show Voltage = não; Frequency (Hz) = 60; Phase Offset (degrees) = 0 |
| 1 | `VoltageElm` | A/C source | Max Voltage = 169.7; Waveform = A/C; DC Offset (V) = 0; Internal Resistance (ohms) = 0; Show Voltage = não; Frequency (Hz) = 60; Phase Offset (degrees) = 120 |
| 2 | `VoltageElm` | A/C source | Max Voltage = 169.7; Waveform = A/C; DC Offset (V) = 0; Internal Resistance (ohms) = 0; Show Voltage = não; Frequency (Hz) = 60; Phase Offset (degrees) = 240 |
| 3 | `ResistorElm` | resistor | Resistance (ohms) = 1k |
| 5 | `ResistorElm` | resistor | Resistance (ohms) = 1k |
| 6 | `ResistorElm` | resistor | Resistance (ohms) = 1k |
| 13 | `SwitchElm` | switch (SPST) | Momentary Switch = não; IEC Symbol = não; Label (for linking) = ; Keyboard Shortcut = ; On Resistance (ohms) = 0 |
| 14 | `TextElm` | text | Text = Phase A; Size = 12; Draw Bar On Top = não; Color = #c0c0c0 |
| 15 | `TextElm` | text | Text = Phase C; Size = 12; Draw Bar On Top = não; Color = #c0c0c0 |
| 16 | `TextElm` | text | Text = Phase B; Size = 12; Draw Bar On Top = não; Color = #c0c0c0 |
| 17 | `TextElm` | text | Text = Neutral-Return; Size = 12; Draw Bar On Top = não; Color = #c0c0c0 |
| 18 | `TextElm` | text | Text = 3 Phase Wye-to-Wye Balanced Resistive Load; Size = 24; Draw Bar On Top = não; Color = #c0c0c0 |
<!-- componentes:fim -->

## Símbolos e parâmetros

Os símbolos usados neste documento (definições completas no [glossário](../glossario-simbolos.md)):

| Símbolo | Nome | Significado | Unidade |
|---------|------|-------------|---------|
| $p$ | índice p | índice inteiro do somatório ou do harmônico | — |
| $t$ | tempo | instante, contado a partir do início da simulação | s |
| $R$ | resistência | oposição de um resistor à passagem da corrente ($V = R I$) | Ω |
| $v_A$ | tensão da fase A | tensão da fase A de um sistema trifásico | V |
| $V_p$ | tensão de pico | valor máximo atingido por uma senoide | V |
| $\omega$ | frequência angular | $2\pi f$: a frequência medida em radianos por segundo | rad/s |
| $v_B$ | tensão instantânea da base | tensão da base em relação à terra em cada instante | V |
| $v_C$ | tensão no capacitor | tensão entre os terminais do capacitor | V |
| $i_k$ | corrente da fase k | corrente da fase de índice $k$ | A |
| $v_k$ | tensão da fase k | tensão da fase de índice $k$ | V |
| $i_N$ | corrente do neutro | soma das correntes das três fases | A |
| $i_A$ | corrente da fase A | corrente da fase A de um sistema trifásico | A |
| $i_B$ | corrente da fase B | corrente da fase B de um sistema trifásico | A |
| $i_C$ | corrente da fase C | corrente da fase C de um sistema trifásico | A |
| $V_{\text{linha}}$ | tensão de linha | tensão entre duas fases de um sistema trifásico | V |
| $V_{\text{ef}}$ | tensão eficaz | valor eficaz (RMS) da tensão | V |

## Teoria

As três tensões de fase são

$$v_A = V_p\sin\omega t,\qquad v_B = V_p\sin(\omega t - 120^\circ),\qquad v_C = V_p\sin(\omega t - 240^\circ),$$

e sua soma é nula em todos os instantes ($v_A + v_B + v_C = 0$). Com cargas iguais $R$, as correntes de fase $i_k = v_k/R$ também somam zero, e a corrente no fio neutro,

$$i_N = i_A + i_B + i_C = 0.$$

Por isso, num sistema equilibrado o fio neutro não conduz corrente (ele só carrega o desequilíbrio entre as cargas), e retirá-lo não muda as tensões das cargas.

A tensão entre duas fases (tensão de linha) é $\sqrt3$ vezes a de fase: $V_{linha} = \sqrt3\cdot 120 = 208\ \text{V}$ eficazes. A potência total em cargas equilibradas é constante no tempo: $p = 3\thinspace V_{ef}^2/R = 43{,}2\ \text{W}$, uma das vantagens do sistema trifásico para acionar motores.

## Resultados esperados

Cada resistor recebe $120\ \text{V}$ eficazes e uma corrente de $120\ \text{mA}$ eficazes ($169{,}7\ \text{mA}$ de pico), defasadas de $120^\circ$ entre si; a corrente no neutro é zero, com a chave fechada e com ela aberta. As verificações conferem a tensão eficaz dos três resistores e a corrente nula do neutro com a chave fechada (posição inicial) e aberta, numa janela de um ciclo (de $0{,}05$ a $0{,}0667\ \text{s}$), com tolerância de 1% (o simulador fica a menos de 0,1% da teoria).

## O que observar

- Abra e feche a chave do neutro: nada muda, porque a corrente que ele carregaria é nula.
- Os osciloscópios mostram três senoides defasadas de um terço de período.
- Mude uma das cargas: o sistema deixa de ser equilibrado, e a corrente do neutro deixa de ser nula (nessa situação, abrir o neutro altera as tensões das cargas).

## Limites do modelo

Fontes e cargas ideais, sem impedância de linha; as fontes são perfeitamente iguais e defasadas de exatamente $120^\circ$. Num sistema real há desequilíbrios e harmônicos (os de terceira ordem, em particular, somam-se no neutro em vez de se anularem).

## Referências

- C. K. Alexander e M. N. O. Sadiku, *Fundamentos de Circuitos Elétricos*, AMGH.
- J. W. Nilsson e S. A. Riedel, *Circuitos Elétricos*, Pearson.

O circuito deste exemplo faz parte da coleção do simulador original de Paul Falstad e Iain Sharp (CircuitJS1); a explicação é deste projeto.
