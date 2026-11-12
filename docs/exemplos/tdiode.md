---
exemplo: tdiode.txt
titulo: I/V Curve
categoria: Misc Devices > Tunnel Diodes
verificacoes:
  - t=0.002 elemento=2:TunnelDiodeElm grandeza=i esperado=0.004708 tol=0.01
  - t=0.005 elemento=2:TunnelDiodeElm grandeza=i esperado=0.002547 tol=0.01
  - t=0.01 elemento=2:TunnelDiodeElm grandeza=i esperado=0.008774 tol=0.01
  - t=0.0145 elemento=2:TunnelDiodeElm grandeza=i esperado=0.002148 tol=0.01
---

# Curva I–V do diodo túnel

<!-- navegacao:inicio -->
[Índice](README.md) · [Próximo: LC Oscillator](tdosc.md)
<!-- navegacao:fim -->

## Objetivo

Mostrar a **curva característica** do diodo túnel, com sua região de resistência negativa: uma tensão triangular varre o diodo e o osciloscópio desenha a corrente em função da tensão.

## O circuito

<!-- esquema:inicio -->
![Esquema do circuito](img/tdiode.svg)
<!-- esquema:fim -->

Uma fonte triangular de $-0{,}02\ \text{V}$ a $0{,}54\ \text{V}$ ($0{,}26\ \text{V}$ de centro e $0{,}28\ \text{V}$ de amplitude), a $50\ \text{Hz}$, está ligada diretamente ao diodo túnel, que fecha o circuito pela terra. A tensão no diodo é a da fonte. Dois osciloscópios acompanham o diodo.

<!-- componentes:inicio -->
| N.º | Tipo | Elemento | Valores |
|----:|------|----------|---------|
| 0 | `RailElm` | triangle gen | Max Voltage = 280m; Waveform = Triangle; DC Offset (V) = 260m; Internal Resistance (ohms) = 0; Show Voltage = não; Frequency (Hz) = 50; Phase Offset (degrees) = 0 |
| 1 | `GroundElm` | ground | Symbol = Earth |
| 2 | `TunnelDiodeElm` | tunnel diode |  |
<!-- componentes:fim -->

## Teoria

O diodo túnel do simulador tem a corrente

$$I(V) = I_P\,\frac{V}{V_P}\,e^{1 - V/V_P} + I_P\,e^{-V_{PP}/V_T}\left(e^{V/V_T} - 1\right) + I_V\left(e^{V - V_V} - e^{-V_V}\right),$$

com $V_P = 0{,}1\ \text{V}$, $I_P = 4{,}7\ \text{mA}$, $V_V = 0{,}37\ \text{V}$, $I_V = 370\ \mu\text{A}$, $V_T = 26\ \text{mV}$ e $V_{PP} = 0{,}525\ \text{V}$. O primeiro termo é o de tunelamento (pico), o segundo, a corrente de difusão usual (Shockley) e o terceiro, uma corrente excedente que cresce de novo depois do vale. A curva tem um **pico** de $4{,}73\ \text{mA}$ em $0{,}10\ \text{V}$ e um **vale** de $1{,}00\ \text{mA}$ em $0{,}43\ \text{V}$; entre os dois, a inclinação é negativa: é a região de resistência diferencial negativa.

## Resultados esperados

O simulador dá, ao longo da rampa de subida de $10\ \text{ms}$:

| $t$ | $V$ | $I$ |
|---:|---:|---:|
| $2\ \text{ms}$ | $0{,}092\ \text{V}$ | $4{,}708\ \text{mA}$ (junto ao pico) |
| $5\ \text{ms}$ | $0{,}260\ \text{V}$ | $2{,}547\ \text{mA}$ (região negativa) |
| $10\ \text{ms}$ | $0{,}540\ \text{V}$ | $8{,}774\ \text{mA}$ (fim da rampa) |

Na rampa de descida, para $V = 0{,}288\ \text{V}$ a corrente é de $2{,}148\ \text{mA}$ (a curva é a mesma: o diodo não tem memória).

## O que observar

Veja os osciloscópios: a corrente sobe ao pico em $0{,}1\ \text{V}$, cai até o vale em $0{,}43\ \text{V}$ e volta a subir. Entre o pico e o vale um acréscimo de tensão diminui a corrente, resistência diferencial negativa que os exemplos seguintes usam para fazer oscilar circuitos.

## Limites do modelo

- O modelo é o da curva estática; um diodo túnel real tem capacitância de junção, resistência série e indutância dos terminais, que limitam a frequência de operação.
- Os parâmetros do modelo ($4{,}7\ \text{mA}$ de pico, $0{,}1\ \text{V}$...) são fixos; diodos túnel reais variam de unidade para unidade.

## Referências

Sze e Ng, *Physics of Semiconductor Devices*, capítulo sobre o diodo túnel; Esaki, *New phenomenon in narrow germanium p-n junctions*, Physical Review 109 (1958).

O circuito deste exemplo faz parte da coleção do simulador original de Paul Falstad e Iain Sharp (CircuitJS1); a explicação é deste projeto.
