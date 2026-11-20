---
exemplo: cc2.txt
titulo: CCII+
categoria: Misc Devices > Current Conveyor
verificacoes:
  - t=0.001 elemento=0:CC2Elm grandeza=v0 esperado=4.5 tol=0.001
  - t=0.001 elemento=0:CC2Elm grandeza=v1 esperado=4.5 tol=0.001
  - t=0.001 elemento=0:CC2Elm grandeza=v2 esperado=0.75 tol=0.001
  - t=0.001 elemento=5:ResistorElm grandeza=i esperado=0.0075 tol=0.001
  - t=0.001 elemento=1:ResistorElm grandeza=i esperado=0.0075 tol=0.001
---

# Transportador de corrente CCII+

<!-- navegacao:inicio -->
[Índice](README.md) · [Próximo: CCII-](cc2n.md)
<!-- navegacao:fim -->

## Objetivo

Mostrar o transportador de corrente CCII$+$: a tensão de $Y$ aparece em $X$, a corrente de $X$ é copiada para $Z$ e a carga em $Z$ vê uma corrente que não depende dela.

## O circuito

<!-- esquema:inicio -->
![Esquema do circuito](img/cc2.svg)
<!-- esquema:fim -->

O terminal $Y$ recebe uma tensão ajustável de $0$ a $5\ \text{V}$ (*Y Voltage*, em $4{,}5\ \text{V}$). O terminal $X$ alimenta, a partir de $Y$, um potenciômetro de $1\ \text{k}\Omega$ (*X Resistance*, ajustado em $50\ \%$) em série com um resistor de $100\ \Omega$ para a terra. A saída $Z$ alimenta um resistor de $100\ \Omega$ para a terra.

<!-- componentes:inicio -->
| N.º | Tipo | Elemento | Valores |
|----:|------|----------|---------|
| 0 | `CC2Elm` | CC2 |  |
| 1 | `ResistorElm` | resistor | Resistance (ohms) = 100 |
| 2 | `GroundElm` | ground | Symbol = Earth |
| 3 | `VarRailElm` | voltage source | Min Voltage = 0; Max Voltage = 5; Slider Text = Y Voltage |
| 4 | `PotElm` | potentiometer | Resistance (ohms) = 1k; Slider Text = X Resistance; Show Values = não; Group Number (for linking) = 0 |
| 5 | `ResistorElm` | resistor | Resistance (ohms) = 100 |
| 6 | `GroundElm` | ground | Symbol = Earth |
<!-- componentes:fim -->

## Símbolos e parâmetros

Os símbolos usados neste documento (definições completas no [glossário](../glossario-simbolos.md)):

| Símbolo | Nome | Significado | Unidade |
|---------|------|-------------|---------|
| $Y$ | terminal Y | entrada de tensão do conversor de corrente (corrente nula no ideal) | — |
| $X$ | terminal X | entrada de corrente do conversor de corrente, que copia a tensão de $Y$ | — |
| $Z$ | impedância | oposição de um elemento à corrente alternada, na frequência considerada | Ω |
| $V_X$ | tensão em X | tensão do terminal $X$, que segue $V_Y$ | V |
| $V_Y$ | tensão em Y | tensão do terminal $Y$ do conversor de corrente | V |
| $I_Y$ | corrente em Y | corrente que entra no terminal $Y$ (nula no conversor ideal) | A |
| $I_Z$ | corrente em Z | corrente que sai do terminal $Z$, igual a $\pm I_X$ | A |
| $I_X$ | corrente em X | corrente que entra no terminal $X$ do conversor de corrente | A |
| $R_{\text{pot}}$ | resistência do potenciômetro | resistência total do potenciômetro | Ω |
| $V_Z$ | tensão zener | tensão em que o diodo zener regula | V |
| $R_Z$ | resistência em Z | resistor ligado ao terminal $Z$ | Ω |

## Teoria

O **transportador de corrente** de segunda geração (CCII, *second-generation current conveyor*) tem três terminais: $X$, $Y$ e $Z$. Idealmente,

$$V_X = V_Y,\qquad I_Y = 0,\qquad I_Z = \pm I_X,$$

isto é, a tensão em $Y$ (de entrada de alta impedância) é copiada em $X$ (de baixa impedância), e a corrente que flui em $X$ é copiada para $Z$ (saída de alta impedância), com o mesmo sinal no CCII$+$ e o oposto no CCII$-$. O elemento do simulador tem ganho de corrente $\pm 1$.

Aqui $V_X = V_Y = 4{,}5\ \text{V}$ e a corrente em $X$ é $I_X = V_X/(R_{pot} + 100\ \Omega) = 4{,}5\ \text{V}/(500 + 100)\ \Omega = 7{,}5\ \text{mA}$, copiada para $Z$: $V_Z = I_Z R_Z = 7{,}5\ \text{mA}\cdot 100\ \Omega = 0{,}75\ \text{V}$.

## Resultados esperados

O simulador dá $V_Y = V_X = 4{,}500\ \text{V}$, $I_X = 7{,}500\ \text{mA}$, $I_Z = 7{,}500\ \text{mA}$ e $V_Z = 0{,}750\ \text{V}$, exatamente os valores da teoria.

## O que observar

Mude a resistência de $X$: a corrente $I_X$ muda e $I_Z$ a acompanha, mas a tensão em $X$ continua igual à de $Y$. Mude a carga em $Z$: $I_Z$ não muda, só $V_Z$.

## Limites do modelo

- O elemento é ideal: $I_Y = 0$, resistência de saída em $Z$ infinita e resistência em $X$ nula; um CCII real tem uma resistência em $X$ de alguns ohms e uma resistência finita em $Z$.
- O elemento não tem limite de banda.

## Referências

Sedra e Smith, *A second generation current conveyor and its applications*, IEEE Transactions on Circuit Theory 17 (1970); Toumazou, Lidgey e Haigh, *Analogue IC Design: The Current-Mode Approach*.

O circuito deste exemplo faz parte da coleção do simulador original de Paul Falstad e Iain Sharp (CircuitJS1); a explicação é deste projeto.
