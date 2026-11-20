---
exemplo: darlington.txt
titulo: Darlington Pair
categoria: Transistors
verificacoes:
  - t=0.01 elemento=4:ResistorElm grandeza=i esperado=0 abs=0.000001
  - t=0.01 elemento=4:ResistorElm grandeza=i esperado=0.013965 tol=0.01 alterna=2
  - t=0.01 elemento=1:ResistorElm grandeza=i esperado=0.0000019 tol=0.05 alterna=2
---

# Par Darlington

<!-- navegacao:inicio -->
[Índice](README.md) · [Anterior: Current Mirror](mirror.md) · [Próximo: Gilbert Cell Multiplier](gilbertcell.md)
<!-- navegacao:fim -->

## Objetivo

Mostrar o par Darlington: dois transistores em cascata, em que o emissor do primeiro alimenta a base do segundo, formam um "transistor" de ganho de corrente muito alto, o produto dos ganhos.

## O circuito

<!-- esquema:inicio -->
![Esquema do circuito](img/darlington.svg)
<!-- esquema:fim -->

Uma fonte de $5\ \text{V}$ alimenta uma carga de $300\ \Omega$ no coletor de um par Darlington de dois NPN ($\beta = 100$ cada). A base do primeiro recebe corrente por um resistor de $2\ \text{M}\Omega$ e uma chave (aberta no início).

<!-- componentes:inicio -->
| N.º | Tipo | Elemento | Valores |
|----:|------|----------|---------|
| 0 | `NTransistorElm` | transistor | Beta/hFE = 100; Swap E/C = não; Draw Circle = não; Model = default |
| 1 | `ResistorElm` | resistor | Resistance (ohms) = 2M |
| 2 | `SwitchElm` | switch (SPST) | Momentary Switch = não; IEC Symbol = não; Label (for linking) = ; Keyboard Shortcut = ; On Resistance (ohms) = 0 |
| 4 | `ResistorElm` | resistor | Resistance (ohms) = 300 |
| 6 | `NTransistorElm` | transistor | Beta/hFE = 100; Swap E/C = não; Draw Circle = não; Model = default |
| 11 | `RailElm` | voltage source | Voltage = 5; Waveform = D/C; DC Offset (V) = 0; Internal Resistance (ohms) = 0 |
| 12 | `GroundElm` | ground | Symbol = Earth |
<!-- componentes:fim -->

## Símbolos e parâmetros

Os símbolos usados neste documento (definições completas no [glossário](../glossario-simbolos.md)):

| Símbolo | Nome | Significado | Unidade |
|---------|------|-------------|---------|
| $t$ | tempo | instante, contado a partir do início da simulação | s |
| $\beta$ | ganho de corrente | razão $I_C/I_B$ do transistor bipolar | — |
| $V_{BE}$ | tensão base-emissor | tensão entre a base e o emissor | V |
| $V_{BC}$ | tensão base-coletor | tensão entre a base e o coletor | V |
| $I_C$ | corrente de coletor | corrente que entra pelo coletor do transistor bipolar | A |
| $I_s$ | corrente de saturação | corrente reversa de um diodo ou de uma junção | A |
| $V_T$ | tensão térmica | $kT/q$: cerca de $25{,}85\ \text{mV}$ a $300\ \text{K}$ | V |
| $\beta_R$ | ganho de corrente reverso | $\beta$ do transistor ligado ao contrário (coletor e emissor trocados) | — |
| $I_B$ | corrente de base | corrente que entra pela base do transistor bipolar | A |
| $g_m$ | transcondutância | quanto a corrente de saída varia por volt na entrada, $\partial I_D/\partial V_{GS}$ | S |
| $r_e$ | resistência de emissor | resistência dinâmica do emissor, $V_T/I_E$ | Ω |
| $I_E$ | corrente de emissor | corrente que sai pelo emissor do transistor bipolar | A |
| $I_{E1}$ | corrente de emissor 1 | corrente que sai pelo emissor do transistor bipolar (do elemento de número 1) | A |
| $\beta_1$ | ganho de corrente do transistor 1 | ganho de corrente $\beta$ do transistor de número 1 | — |
| $I_{B1}$ | corrente de base 1 | corrente que entra pela base do transistor bipolar (do elemento de número 1) | A |
| $\beta_2$ | ganho de corrente do transistor 2 | ganho de corrente $\beta$ do transistor de número 2 | — |
| $\beta_{\text{eq}}$ | ganho de corrente equivalente | ganho de corrente do conjunto de transistores (no Darlington, $\beta_1\beta_2$) | — |
| $V_{CE(\text{sat})2}$ | tensão de saturação do segundo transistor | $V_{CE}$ do transistor de saída do par Darlington quando saturado | V |
| $V_{CE}$ | tensão coletor-emissor | tensão entre o coletor e o emissor | V |

## Teoria

O transistor bipolar do simulador (modelo "default") segue o modelo de Ebers–Moll, com $V_{BE}$ e $V_{BC}$ as tensões base-emissor e base-coletor (para um NPN):

$$I_C = I_s\left(e^{V_{BE}/V_T} - e^{V_{BC}/V_T}\right) - \frac{I_s}{\beta_R}\left(e^{V_{BC}/V_T} - 1\right),\qquad I_B = \frac{I_s}{\beta}\left(e^{V_{BE}/V_T} - 1\right) + \frac{I_s}{\beta_R}\left(e^{V_{BC}/V_T} - 1\right),$$

com $I_s = 10^{-13}\ \text{A}$, $\beta = 100$, $\beta_R = 1$ e $V_T = 25{,}865\ \text{mV}$; sem efeito Early, sem capacitâncias e sem resistências série. Na **região ativa** ($V_{BE} \approx 0{,}65$ a $0{,}75\ \text{V}$, junção base-coletor com polarização inversa) isso se reduz a $I_C = \beta I_B = I_s\thinspace e^{V_{BE}/V_T}$, com transcondutância $g_m = I_C/V_T$ e resistência de emissor $r_e = V_T/I_E$. O PNP é o complementar, com todas as tensões e correntes invertidas.

## Resultados esperados

A corrente de emissor do primeiro transistor, $I_{E1} = (\beta_1 + 1)I_{B1}$, é a corrente de base do segundo; a corrente total de coletor é

$$I_C \approx \beta_1\beta_2\thinspace I_{B1} = \beta_{eq}\thinspace I_{B1},\qquad \beta_{eq} \approx \beta_1\beta_2 = 10^4.$$

A tensão de entrada necessária é $V_{BE} \approx 2 \times 0{,}65\ \text{V} = 1{,}3\ \text{V}$ (duas junções em série). A impedância de entrada é muito alta ($\beta_{eq}$ vezes a de emissor), e a de saturação é maior (a saturação do conjunto é $V_{CE(sat)2} + V_{BE}$ do primeiro...). Serve para acionar cargas fortes com corrente de entrada minúscula.

## O que observar

Com a chave fechada, a corrente de base é de apenas $1{,}84\ \mu\text{A}$ (da fonte de $5\ \text{V}$ por $2\ \text{M}\Omega$, menos $V_{BE}$ total de $1{,}21\ \text{V}$) e a carga recebe $13{,}97\ \text{mA}$: ganho de corrente de $7\thinspace600$ (menor que $10^4$ porque o segundo transistor está perto da saturação, $V_{CE} = 0{,}81\ \text{V}$). Com a chave aberta, a corrente na carga é nula ($\approx 0{,}1\ \mu\text{A}$ de vazamento do modelo). As verificações conferem a corrente da carga com a chave aberta e fechada e a de base, em $t = 10\ \text{ms}$ (tolerância de 1% a 5%; o simulador fica a menos de $0{,}1\%$ da integração independente).

## Limites do modelo

- Feche a chave: uma corrente de base de $2\ \mu\text{A}$ comanda $14\ \text{mA}$.
- Compare com um único transistor, que precisaria de $140\ \mu\text{A}$ na base para a mesma corrente.
- A tensão de entrada é o dobro da de um transistor: $\approx 1{,}2\ \text{V}$.

## Referências

O Darlington é mais lento (o primeiro transistor não tem um caminho para descarregar a base do segundo, e o par leva mais tempo para desligar) e tem tensão de saturação maior (cerca de $0{,}8\ \text{V}$ aqui). O modelo não tem capacitâncias.

O circuito deste exemplo faz parte da coleção do simulador original de Paul Falstad e Iain Sharp (CircuitJS1); a explicação é deste projeto.
