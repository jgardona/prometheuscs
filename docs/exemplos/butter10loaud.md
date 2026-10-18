---
exemplo: butter10loaud.txt
titulo: Butterworth Low-Pass w/ Noise
categoria: Passive Filters
sem-verificacao: entrada de ruído aleatório; o resultado é ouvido, não repetível
---

# Passa-baixas de Butterworth de 10 polos com ruído

<!-- navegacao:inicio -->
[Índice](README.md) · [Anterior: Butterworth High-Pass (10 pole)](butter10hi.md) · [Próximo: Butterworth High-Pass w/ Noise](butter10hiaud.md)
<!-- navegacao:fim -->

## Objetivo

Ouvir o efeito de um filtro passa-baixas de décima ordem sobre ruído branco: só os graves passam, e o som fica "abafado".

## O circuito

<!-- esquema:inicio -->
![Esquema do circuito](img/butter10loaud.svg)
<!-- esquema:fim -->

A mesma escada LC de dez polos do exemplo do passa-baixas de Butterworth (carga de $50\ \Omega$), alimentada por um gerador de ruído de 5 V; a saída vai a uma saída de áudio (1 s, 8000 amostras por segundo).

<!-- componentes:inicio -->
| N.º | Tipo | Elemento | Valores |
|----:|------|----------|---------|
| 0 | `CapacitorElm` | capacitor | Capacitance (F) = 5.9μ; Trapezoidal Approximation = sim; Initial Voltage (on Reset) = 1m; Series Resistance = 0 |
| 1 | `InductorElm` | inductor | Inductance (H) = 12.4m; Trapezoidal Approximation = sim; Initial Current (on Reset) (A) = 0; Saturation Current (A) (0=none) = 0 |
| 3 | `ResistorElm` | resistor | Resistance (ohms) = 50 |
| 4 | `GroundElm` | ground | Symbol = Earth |
| 5 | `GroundElm` | ground | Symbol = Earth |
| 6 | `CapacitorElm` | capacitor | Capacitance (F) = 5.4μ; Trapezoidal Approximation = sim; Initial Voltage (on Reset) = 1m; Series Resistance = 0 |
| 7 | `InductorElm` | inductor | Inductance (H) = 14.4m; Trapezoidal Approximation = sim; Initial Current (on Reset) (A) = 0; Saturation Current (A) (0=none) = 0 |
| 8 | `GroundElm` | ground | Symbol = Earth |
| 9 | `CapacitorElm` | capacitor | Capacitance (F) = 4.1μ; Trapezoidal Approximation = sim; Initial Voltage (on Reset) = 1m; Series Resistance = 0 |
| 10 | `InductorElm` | inductor | Inductance (H) = 12m; Trapezoidal Approximation = sim; Initial Current (on Reset) (A) = 0; Saturation Current (A) (0=none) = 0 |
| 11 | `GroundElm` | ground | Symbol = Earth |
| 12 | `CapacitorElm` | capacitor | Capacitance (F) = 2.4μ; Trapezoidal Approximation = sim; Initial Voltage (on Reset) = 1m; Series Resistance = 0 |
| 13 | `InductorElm` | inductor | Inductance (H) = 8.3m; Trapezoidal Approximation = sim; Initial Current (on Reset) (A) = 0; Saturation Current (A) (0=none) = 0 |
| 14 | `GroundElm` | ground | Symbol = Earth |
| 15 | `CapacitorElm` | capacitor | Capacitance (F) = 497.9n; Trapezoidal Approximation = sim; Initial Voltage (on Reset) = 1m; Series Resistance = 0 |
| 16 | `InductorElm` | inductor | Inductance (H) = 3.7m; Trapezoidal Approximation = sim; Initial Current (on Reset) (A) = 0; Saturation Current (A) (0=none) = 0 |
| 17 | `GroundElm` | ground | Symbol = Earth |
| 18 | `RailElm` | noise gen | Max Voltage = 5; Waveform = Noise; DC Offset (V) = 0; Internal Resistance (ohms) = 0; Show Voltage = não |
| 19 | `AudioOutputElm` | audio output | Duration (s) = 1; Sampling Rate = 8000 |
<!-- componentes:fim -->

## Teoria

O ruído branco tem densidade espectral de potência uniforme; a saída de um filtro linear tem densidade $S_{out}(f) = |H(f)|^2 S_{in}(f)$, de modo que o filtro "esculpe" o espectro do ruído e o ouvido percebe só a faixa que passa.

A resposta de módulo de um filtro de Butterworth de ordem $n$ é "maximamente plana":

$$|H(f)| = \frac{1}{\sqrt{1 + (f/f_c)^{2n}}}.$$

Em $f_c$ vale $1/\sqrt2$ ($-3\ \text{dB}$); acima de $f_c$ o ganho cai $20n\ \text{dB}$ por década (para $n=10$, $200\ \text{dB}$ por década), e abaixo dele a curva é praticamente plana. Uma escada LC com $n$ elementos reativos realiza essa resposta com valores de componentes dados por $g_k = 2\sin\dfrac{(2k-1)\pi}{2n}$.

## Resultados esperados

A frequência de corte é a do exemplo da varredura ($\approx 1\ \text{kHz}$), pois os componentes são os mesmos. Como a entrada é aleatória, nenhum valor numérico se repete de uma execução para outra; o resultado é para ser ouvido.

## O que observar

- Ouça a saída: ruído grave, sem os agudos, cortado de forma abrupta acima de 1 kHz.
- Compare com o passa-altas com ruído.

## Limites do modelo

Gerador de ruído e componentes ideais. A saída é áudio: não há verificação numérica porque a entrada é aleatória.

## Referências

- A. S. Sedra e K. C. Smith, *Microeletrônica*, Pearson.
- P. Horowitz e W. Hill, *The Art of Electronics*, Cambridge University Press.

O circuito deste exemplo faz parte da coleção do simulador original de Paul Falstad e Iain Sharp (CircuitJS1); a explicação é deste projeto.
