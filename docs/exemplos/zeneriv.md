---
exemplo: zeneriv.txt
titulo: I/V Curve
categoria: Diodes > Zener Diodes
verificacoes:
  - t=0.06 de=0.04 elemento=2:ZenerElm grandeza=vd fn=max esperado=1.23 tol=0.01
  - t=0.06 de=0.04 elemento=2:ZenerElm grandeza=vd fn=min esperado=-5.93 tol=0.01
  - t=0.06 de=0.04 elemento=2:ZenerElm grandeza=i fn=max esperado=3634.57 tol=0.02
  - t=0.06 de=0.04 elemento=2:ZenerElm grandeza=i fn=min esperado=-1737.5 tol=0.02
---

# Curva I×V do diodo Zener

<!-- navegacao:inicio -->
[Índice](README.md) · [Próximo: Voltage Reference](zenerref.md)
<!-- navegacao:fim -->

## Objetivo

Mostrar a característica de um diodo Zener: conduz como um diodo comum na polarização direta e, na inversa, conduz fortemente a partir da tensão de ruptura (aqui, $5{,}6\ \text{V}$).

## O circuito

<!-- esquema:inicio -->
![Esquema do circuito](img/zeneriv.svg)
<!-- esquema:fim -->

Um gerador de onda triangular de $50\ \text{Hz}$ varre a tensão de $-5{,}93\ \text{V}$ a $+1{,}23\ \text{V}$ sobre um diodo Zener de $5{,}6\ \text{V}$, ligado diretamente (sem resistor). Dois osciloscópios mostram a corrente (em escala logarítmica) e a curva $I\times V$.

<!-- componentes:inicio -->
| N.º | Tipo | Elemento | Valores |
|----:|------|----------|---------|
| 0 | `RailElm` | triangle gen | Max Voltage = 3.6; Waveform = Triangle; DC Offset (V) = -2.4; Internal Resistance (ohms) = 0; Show Voltage = não; Frequency (Hz) = 50; Phase Offset (degrees) = 0 |
| 1 | `GroundElm` | ground | Symbol = Earth |
| 2 | `ZenerElm` | Zener diode | Model = default-zener |
<!-- componentes:fim -->

## Teoria

Na polarização direta, o Zener é um diodo comum (equação de Shockley com $I_s = 1{,}714\cdot 10^{-7}\ \text{A}$, $n = 2$, $V_T = 25{,}865\ \text{mV}$). Na inversa, o modelo soma um segundo termo exponencial, bem mais inclinado, que representa a ruptura:

$$i = I_s\left(e^{v/(nV_T)} - e^{(-v - v_0)/V_T} - 1\right)\quad (v < 0),$$

com o deslocamento $v_0$ escolhido para que $i = -5\ \text{mA}$ na tensão de Zener $V_Z = 5{,}6\ \text{V}$: $v_0 = V_Z - V_T\ln(5\ \text{mA}/I_s - 1) = 5{,}334\ \text{V}$. Acima da ruptura a corrente varia $e$ vezes a cada $V_T = 25{,}9\ \text{mV}$, e a resistência dinâmica é $r_Z \approx V_T/|i|$, só $5{,}2\ \Omega$ em $5\ \text{mA}$.

## Resultados esperados

Cálculo pela equação (o simulador reproduz os valores a menos de $10^{-6}$):

| $v$ | $i$ |
|----:|----:|
| $+1{,}23\ \text{V}$ | $+3\,634{,}6\ \text{A}$ |
| $-5{,}0\ \text{V}$ | $-0{,}17\ \mu\text{A}$ |
| $-5{,}4\ \text{V}$ | $-2{,}4\ \mu\text{A}$ |
| $-5{,}5\ \text{V}$ | $-0{,}105\ \text{mA}$ |
| $-5{,}6\ \text{V}$ | $-5{,}00\ \text{mA}$ |
| $-5{,}7\ \text{V}$ | $-239\ \text{mA}$ |
| $-5{,}93\ \text{V}$ | $-1\,737{,}5\ \text{A}$ |

A corrente é nula até perto de $-5{,}3\ \text{V}$ e depois sobe de modo abrupto: o "joelho" da ruptura. As correntes de milhares de ampères nas pontas da varredura são só consequência de o gerador ser ideal e de não haver resistor limitador. As verificações conferem os extremos da tensão e da corrente (tolerância de 1% e 2%).

## O que observar

- No gráfico logarítmico a corrente reversa é praticamente plana até perto de $5{,}3\ \text{V}$ e depois dispara: o Zener "prende" a tensão em torno de $5{,}6\ \text{V}$.
- A parte direta é a de um diodo comum, com o joelho em cerca de $0{,}6\ \text{V}$.
- No exemplo seguinte um resistor limita a corrente e o Zener é usado como referência de tensão.

## Limites do modelo

A ruptura do modelo é um termo exponencial com a inclinação de $V_T$ (sem o coeficiente de emissão $n$); a resistência dinâmica de um Zener real de $5{,}6\ \text{V}$ costuma ser de poucos ohms a dezenas de ohms, e o coeficiente de temperatura é próximo de zero perto de $5{,}6\ \text{V}$. Um Zener real queimaria muito antes dos milhares de ampères aqui mostrados.

## Referências

- A. S. Sedra e K. C. Smith, *Microeletrônica*, Pearson.
- P. Horowitz e W. Hill, *The Art of Electronics*, Cambridge University Press.

O circuito deste exemplo faz parte da coleção do simulador original de Paul Falstad e Iain Sharp (CircuitJS1); a explicação é deste projeto.
