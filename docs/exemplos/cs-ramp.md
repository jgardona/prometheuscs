---
exemplo: cs-ramp.txt
titulo: CVS Voltage Ramp
categoria: Controlled Sources
verificacoes:
  - t=0.05 elemento=1:ResistorElm grandeza=vd esperado=2.5 tol=0.01
  - t=0.1 elemento=1:ResistorElm grandeza=vd esperado=5 tol=0.01
  - t=0.2 elemento=1:ResistorElm grandeza=vd esperado=5 tol=0.01
---

# Rampa de tensão (fonte de tensão controlada com função linear por partes)

<!-- navegacao:inicio -->
[Índice](README.md) · [Anterior: CVS Full Rectifier](cs-fullrectifier.md) · [Próximo: CCS Current Adder](cs-currentadder.md)
<!-- navegacao:fim -->

## Objetivo

Mostrar uma fonte de tensão controlada pelo **tempo**: a função $\text{pwl}(t, \ldots)$ define uma tensão linear por partes que passa por pontos dados, o que permite gerar rampas, degraus ou formas de onda arbitrárias.

## O circuito

<!-- esquema:inicio -->
![Esquema do circuito](img/cs-ramp.svg)
<!-- esquema:fim -->

Uma fonte de tensão controlada com a expressão $\text{pwl}(t, 0, 0, 0{,}1, 5)$, que só depende do tempo, alimenta um resistor de $1\ \text{k}\Omega$ ligado à terra.

<!-- componentes:inicio -->
| N.º | Tipo | Elemento | Valores |
|----:|------|----------|---------|
| 0 | `GroundElm` | ground | Symbol = Earth |
| 1 | `ResistorElm` | resistor | Resistance (ohms) = 1k |
| 2 | `GroundElm` | ground | Symbol = Earth |
| 3 | `ControlledVoltageElm` | Controlled Voltage Source | Output Voltage = pwl(t,0,0,.1,5) |
<!-- componentes:fim -->

## Teoria

As fontes controladas de dois terminais (o losango) são definidas por uma **expressão** que lê o resto do circuito **pelo nome**: $v(\text{nome})$ é a tensão de um nó rotulado ou sobre um voltímetro com rótulo, $i(\text{nome})$ a corrente de um amperímetro com rótulo, e $\text{dvdt}$ / $\text{didt}$ as suas taxas de variação. O resultado é a tensão (fonte de tensão controlada, com o $+$ no segundo terminal) ou a corrente (fonte de corrente controlada, no sentido da seta) que a fonte impõe. A função $\text{pwl}(x, x_1, y_1, x_2, y_2, \ldots)$ interpola linearmente entre os pontos $(x_k, y_k)$ e mantém o último valor depois do último ponto. Com $x = t$ e os pontos $(0, 0)$ e $(0{,}1\ \text{s}, 5\ \text{V})$,

$$V(t) = \begin{cases} 50\ \text{V/s}\cdot t, & 0 \le t \le 0{,}1\ \text{s},\\ 5\ \text{V}, & t > 0{,}1\ \text{s}.\end{cases}$$

A corrente no resistor é $V/R$.

## Resultados esperados

| Instante | Tensão (teoria) | Simulador |
|----------|----------------:|----------:|
| $t = 10\ \text{ms}$ | $0{,}5\ \text{V}$ | $0{,}4998\ \text{V}$ |
| $t = 50\ \text{ms}$ | $2{,}5\ \text{V}$ | $2{,}5\ \text{V}$ |
| $t = 100\ \text{ms}$ | $5\ \text{V}$ | $5\ \text{V}$ |
| $t = 200\ \text{ms}$ | $5\ \text{V}$ (constante) | $5\ \text{V}$ |

A diferença de $0{,}0002\ \text{V}$ em $10\ \text{ms}$ é de um passo de tempo da simulação. As verificações conferem os valores em $50$, $100$ e $200\ \text{ms}$.

## O que observar

- A tensão sobe em linha reta durante $0{,}1\ \text{s}$ e depois fica constante em $5\ \text{V}$.
- Acrescente mais pontos à função para gerar qualquer forma de onda (por exemplo, um pulso trapezoidal).
- A rampa pode ser usada como "tensão de teste" para varrer uma característica de um circuito.

## Limites do modelo

O valor depende do tempo de simulação e é determinístico: ao reiniciar, a rampa recomeça. A fonte é ideal, e não limita a corrente que fornece ao resistor.

## Referências

- J. W. Nilsson e S. A. Riedel, *Circuitos Elétricos*, Pearson.
- A. S. Sedra e K. C. Smith, *Microeletrônica*, Pearson.
- P. Falstad, *CircuitJS1 — Controlled Sources* (documentação do simulador).

O circuito deste exemplo faz parte da coleção do simulador original de Paul Falstad e Iain Sharp (CircuitJS1); a explicação é deste projeto.
