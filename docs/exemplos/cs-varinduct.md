---
exemplo: cs-varinduct.txt
titulo: CCS Variable Inductor
categoria: Controlled Sources
verificacoes:
  - t=0.01 elemento=1:ResistorElm grandeza=i esperado=0.013065 tol=0.01
  - t=0.04 elemento=1:ResistorElm grandeza=i esperado=0.0256 tol=0.01
---

# Indutor variável (fonte de tensão controlada)

<!-- navegacao:inicio -->
[Índice](README.md) · [Anterior: CVS Differentiator](cs-diff.md) · [Próximo: CCS Variable Capacitor](cs-varicap.md)
<!-- navegacao:fim -->

## Objetivo

Mostrar como simular um indutor cuja indutância é controlada por uma corrente: uma fonte de tensão controlada com a expressão $\text{didt}(a)\cdot i(b)\cdot 400$ comporta-se como um indutor $L = 400\cdot i_b$, em que `a` é o amperímetro do circuito e `b` o do ramo de controle.

## O circuito

<!-- esquema:inicio -->
![Esquema do circuito](img/cs-varinduct.svg)
<!-- esquema:fim -->

Uma fonte de onda quadrada de $10\ \text{Hz}$ e $\pm 5\ \text{V}$ alimenta, por um resistor de $180\ \Omega$ e pelo amperímetro `a`, a fonte de tensão controlada. Num laço à parte, o amperímetro `b` mede a corrente de uma fonte de $7{,}08\ \text{mA}$ (ajustável por um *slider* chamado "Inductance", de $1$ a $20\ \text{mA}$).

<!-- componentes:inicio -->
| N.º | Tipo | Elemento | Valores |
|----:|------|----------|---------|
| 0 | `DCVoltageElm` | square wave gen | Max Voltage = 5; Waveform = Square Wave; DC Offset (V) = 0; Internal Resistance (ohms) = 0; Show Voltage = não; Specify As = Frequency/Duty Cycle; Frequency (Hz) = 10; Phase Offset (degrees) = 0; Duty Cycle = 50; Rise/Fall Time (s) = 0 |
| 1 | `ResistorElm` | resistor | Resistance (ohms) = 180 |
| 5 | `CurrentElm` | current source | Current (A) = 7.1m; Max Voltage (V, 0=unlimited) = 0 |
| 7 | `AmmeterElm` | Ammeter | Value = Current; Scale = Auto; Circular Symbol = sim; Label = a |
| 8 | `AmmeterElm` | Ammeter | Value = Current; Scale = Auto; Circular Symbol = sim; Label = b |
| 10 | `ControlledVoltageElm` | Controlled Voltage Source | Output Voltage = didt(a)*i(b)*400 |
<!-- componentes:fim -->

## Teoria

As fontes controladas de dois terminais (o losango) são definidas por uma **expressão** que lê o resto do circuito **pelo nome**: $v(\text{nome})$ é a tensão de um nó rotulado ou sobre um voltímetro com rótulo, $i(\text{nome})$ a corrente de um amperímetro com rótulo, e $\text{dvdt}$ / $\text{didt}$ as suas taxas de variação. O resultado é a tensão (fonte de tensão controlada, com o $+$ no segundo terminal) ou a corrente (fonte de corrente controlada, no sentido da seta) que a fonte impõe. Aqui a tensão é $v = \dfrac{di_a}{dt}\cdot i_b\cdot 400$ (com $i_a$ a corrente do circuito e $i_b$ a de controle, em amperes), o que é a lei de um indutor com

$$L = 400\ \frac{\text{H}}{\text{A}}\cdot i_b = 400\cdot 7{,}08\ \text{mA} = 2{,}83\ \text{H}.$$

O circuito é um RL em série, e a corrente em resposta a um degrau $V$ é

$$i(t) = \frac{V}{R}\left(1 - e^{-t/\tau}\right), \qquad \tau = \frac{L}{R} = \frac{2{,}83\ \text{H}}{180\ \Omega} = 15{,}7\ \text{ms}.$$

## Resultados esperados

Em resposta ao primeiro semiciclo positivo ($0$ a $50\ \text{ms}$, com a fonte em $+5\ \text{V}$), com $V/R = 27{,}8\ \text{mA}$:

| Instante | Corrente (teoria) | Simulador |
|----------|------------------:|----------:|
| $t = 10\ \text{ms}$ | $27{,}78\,(1 - e^{-0{,}636}) = 13{,}08\ \text{mA}$ | $13{,}065\ \text{mA}$ |
| $t = 40\ \text{ms}$ | $27{,}78\,(1 - e^{-2{,}54}) = 25{,}6\ \text{mA}$ | $25{,}6\ \text{mA}$ |

As verificações conferem as duas correntes (tolerância de 1%).

## O que observar

- Mova o *slider* "Inductance": a constante de tempo muda e a curva de subida da corrente fica mais lenta (maior corrente de controle, maior $L$) ou mais rápida.
- A "indutância" é proporcional à corrente de controle $i_b$: pode-se simular um indutor saturável, ou ajustável, só mudando a expressão.
- Compare com um indutor real de $2{,}83\ \text{H}$ no mesmo circuito: a resposta é idêntica.

## Limites do modelo

A fonte controlada não é um componente físico: o "indutor" é um modelo matemático ideal, sem resistência série, sem saturação nem capacitância entre espiras. A derivada $\text{didt}$ é calculada numericamente, e mudanças bruscas de $i_b$ durante a simulação produziriam transitórios não físicos.

## Referências

- J. W. Nilsson e S. A. Riedel, *Circuitos Elétricos*, Pearson.
- A. S. Sedra e K. C. Smith, *Microeletrônica*, Pearson.
- P. Falstad, *CircuitJS1 — Controlled Sources* (documentação do simulador).

O circuito deste exemplo faz parte da coleção do simulador original de Paul Falstad e Iain Sharp (CircuitJS1); a explicação é deste projeto.
