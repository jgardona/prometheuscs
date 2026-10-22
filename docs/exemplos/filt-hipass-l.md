---
exemplo: filt-hipass-l.txt
titulo: High-Pass Filter (RL)
categoria: Passive Filters
verificacoes:
  - t=0.0813 de=0.0785 elemento=0:OutputElm grandeza=vd fn=max esperado=3.537 tol=0.03
  - t=0.0985 de=0.0966 elemento=0:OutputElm grandeza=vd fn=max esperado=4.473 tol=0.03
---

# Filtro passa-altas RL

<!-- navegacao:inicio -->
[Índice](README.md) · [Anterior: Low-Pass Filter (RC)](filt-lopass.md) · [Próximo: Low-Pass Filter (RL)](filt-lopass-l.md)
<!-- navegacao:fim -->

## Objetivo

Mostrar um filtro passa-altas de primeira ordem com indutor: o indutor em derivação desvia as baixas frequências para a terra, com frequência de corte $f_c = R/(2\pi L)$.

## O circuito

<!-- esquema:inicio -->
![Esquema do circuito](img/filt-hipass-l.svg)
<!-- esquema:fim -->

Um resistor de $187\ \Omega$ em série com a entrada e um indutor de $65{,}4\ \text{mH}$ para a terra; a saída é a tensão no indutor. O sinal de teste é uma fonte de **varredura de frequência** ("sweep"): uma senoide de 5 V de pico cuja frequência varia de forma logarítmica de 20 Hz a 1 kHz em 100 ms e depois volta (varredura bidirecional). Como a frequência é uma função conhecida do tempo,

$$f(t) = f_{min}\left(\frac{f_{max}}{f_{min}}\right)^{t/T},\qquad 0\le t\le T,$$

o osciloscópio que mostra a tensão de saída ao longo do tempo mostra, na prática, a **resposta em frequência** do filtro: a altura da curva em cada instante é a amplitude de saída na frequência daquele instante.

<!-- componentes:inicio -->
| N.º | Tipo | Elemento | Valores |
|----:|------|----------|---------|
| 0 | `OutputElm` | output | Show Voltage = não |
| 1 | `GroundElm` | ground | Symbol = Earth |
| 2 | `ResistorElm` | resistor | Resistance (ohms) = 187 |
| 3 | `InductorElm` | inductor | Inductance (H) = 65.4m; Trapezoidal Approximation = sim; Initial Current (on Reset) (A) = 0; Saturation Current (A) (0=none) = 0 |
| 4 | `SweepElm` | sweep (log) | Min Frequency (Hz) = 20; Max Frequency (Hz) = 1k; Sweep Time (s) = 100m; Logarithmic = sim; Max Voltage = 5; Bidirectional = sim |
<!-- componentes:fim -->

## Teoria

A saída é o divisor $j\omega L/(R + j\omega L)$:

$$H(j\omega) = \frac{j\omega L/R}{1 + j\omega L/R},\qquad |H| = \frac{\omega L/R}{\sqrt{1+(\omega L/R)^2}}.$$

A constante de tempo é $\tau = L/R$ e a frequência de corte é $f_c = \dfrac{R}{2\pi L} = \dfrac{1}{2\pi\tau}$.

## Resultados esperados

$f_c = 187/(2\pi\cdot65{,}4\times10^{-3}) = 455{,}1\ \text{Hz}$, o mesmo valor do passa-altas RC, e os valores de $|H|$ coincidem com os de lá: 0,707 em $f_c$ ($3{,}54\ \text{V}$ de pico, em $t = 79{,}9\ \text{ms}$) e 0,895 em 910 Hz ($4{,}47\ \text{V}$, em $t = 97{,}6\ \text{ms}$). As verificações medem esses dois pontos (tolerância de 3%).

## O que observar

- A resposta é idêntica à do passa-altas RC: dois circuitos diferentes realizam a mesma função de transferência.
- Aumente $L$: $f_c$ diminui; aumente $R$: $f_c$ aumenta.

## Limites do modelo

Como a frequência varia depressa (uma década a cada 0,0255 s), a amplitude observada num instante é uma aproximação da resposta em regime permanente naquela frequência: nas frequências baixas a variação durante um ciclo não é pequena. Por isso as verificações deste documento usam as frequências mais altas da varredura, em janelas curtas, e uma tolerância de 3%; nos testes o simulador ficou a menos de 2,5% da teoria. Os componentes são ideais. Um indutor real tem resistência do fio, que desloca o corte.

## Referências

- J. W. Nilsson e S. A. Riedel, *Circuitos Elétricos*, Pearson.
- A. S. Sedra e K. C. Smith, *Microeletrônica*, Pearson.

O circuito deste exemplo faz parte da coleção do simulador original de Paul Falstad e Iain Sharp (CircuitJS1); a explicação é deste projeto.
