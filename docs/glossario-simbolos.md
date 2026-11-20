# Glossário de símbolos e parâmetros

Os documentos dos [circuitos de exemplo](exemplos/README.md) usam símbolos como $V_t$, $I_D$ e $V_{GS}$. Aqui está o significado de cada um, com o nome, a unidade e um exemplo que o usa, e, no fim, a explicação de cada campo que aparece nas tabelas de componentes.

Cada documento também tem a sua própria seção **Símbolos e parâmetros**, com as definições dos símbolos que usa. Quando um símbolo tem mais de um significado (por exemplo, $V_t$, a tensão de limiar de um MOSFET, e $V_T$, a tensão térmica), cada significado é uma linha. A coluna *Grafias* lista como o símbolo aparece escrito em texto simples (`Vgs`, `Id`).

## Tensões e correntes

| Símbolo | Grafias | Nome | Significado | Unidade | Exemplo |
|---------|---------|------|-------------|---------|---------|
| $E$ | — | energia | energia armazenada ou dissipada | J | [cap](exemplos/cap.md) |
| $I$ | — | corrente | intensidade da corrente elétrica que passa por um ramo | A | [555saw](exemplos/555saw.md) |
| $i$ | — | corrente instantânea | valor da corrente em cada instante, que pode variar no tempo | A | [amdetect](exemplos/amdetect.md) |
| $I_0$ | — | corrente inicial | valor da corrente no instante inicial | A | [gyrator](exemplos/gyrator.md) |
| $I_1$ | — | corrente 1 | corrente chamada $I_1$ no circuito | A | [ccitov](exemplos/ccitov.md) |
| $i_1$ | — | corrente instantânea 1 | corrente instantânea chamada $i_1$ no circuito | A | [coupled1](exemplos/coupled1.md) |
| $I_{100}$ | — | corrente 100 | corrente chamada $I_{100}$ no circuito | A | [resistors](exemplos/resistors.md) |
| $I_2$ | — | corrente 2 | corrente chamada $I_2$ no circuito | A | [ccitov](exemplos/ccitov.md) |
| $i_2$ | — | corrente instantânea 2 | corrente instantânea chamada $i_2$ no circuito | A | [coupled1](exemplos/coupled1.md) |
| $I_3$ | — | corrente 3 | corrente chamada $I_3$ no circuito | A | [priencoder](exemplos/priencoder.md) |
| $I_{800}$ | — | corrente 800 | corrente chamada $I_{800}$ no circuito | A | [resistors](exemplos/resistors.md) |
| $I_\infty$ | — | corrente final | valor para o qual a corrente tende depois do transitório | A | [induct](exemplos/induct.md) |
| $i_a$ | — | corrente do circuito a | corrente do primeiro circuito | A | [3motor](exemplos/3motor.md) |
| $I_{\text{cc}}$ | — | corrente de curto-circuito | corrente quando os terminais são ligados por um fio | A | [thevenin](exemplos/thevenin.md) |
| $I_{\text{in}}$ | — | corrente de entrada | corrente que entra no circuito pela entrada | A | [cciamp](exemplos/cciamp.md) |
| $i_{\text{in}}$ | — | corrente de entrada | corrente que entra no circuito pela entrada | A | [norton-invert](exemplos/norton-invert.md) |
| $I_k$ | — | corrente do ramo k | corrente no ramo de índice $k$ | A | [r2rladder](exemplos/r2rladder.md) |
| $I_L$ | — | corrente na carga | corrente que passa pela carga | A | [amp-follower](exemplos/amp-follower.md) |
| $I_N$ | — | corrente de Norton | corrente de curto-circuito entre os terminais do circuito | A | [norton](exemplos/norton.md) |
| $I_{\text{out}}$ | — | corrente de saída | corrente que o circuito entrega à saída ou à carga | A | [cs-currentadder](exemplos/cs-currentadder.md) |
| $I_{\text{pico}}$ | — | corrente de pico | maior valor que a corrente atinge no ciclo | A | [capac](exemplos/capac.md) |
| $i_{\text{pico}}$ | — | corrente de pico | maior valor atingido pela corrente | A | [fullrect-sc](exemplos/fullrect-sc.md) |
| $P$ | — | potência | energia transformada por unidade de tempo | W | [3way](exemplos/3way.md) |
| $P_{\text{el}}$ | — | potência elétrica | potência elétrica entregue ao filamento | W | [lightbulb](exemplos/lightbulb.md) |
| $P_L$ | — | potência na carga | potência entregue à carga | W | [tlmatch1](exemplos/tlmatch1.md) |
| $P_l$ | — | potência perdida na linha | potência dissipada na resistência da linha | W | [powerfactor1](exemplos/powerfactor1.md) |
| $P_n$ | — | potência nominal | potência da lâmpada na tensão nominal | W | [lightbulb](exemplos/lightbulb.md) |
| $P_{\text{perda}}$ | — | potência perdida | potência dissipada como calor na linha | W | [lightbulb](exemplos/lightbulb.md) |
| $Q$ | — | potência reativa | potência que vai e volta entre a fonte e o campo do indutor, $V I \sin\varphi$ | var | [powerfactor1](exemplos/powerfactor1.md) |
| $S$ | — | potência aparente | produto $V I$ da tensão pela corrente, em volt-ampère | VA | [powerfactor1](exemplos/powerfactor1.md) |
| $V$ | `V` | tensão | diferença de potencial elétrico entre dois pontos do circuito | V | [555saw](exemplos/555saw.md) |
| $v$ | — | tensão instantânea | valor da tensão em cada instante, que pode variar no tempo | V | [555saw](exemplos/555saw.md) |
| $V_0$ | — | tensão inicial | valor da tensão no instante inicial | V | [cappar](exemplos/cappar.md) |
| $v_0$ | — | tensão inicial | valor da tensão no instante inicial | V | [cap](exemplos/cap.md) |
| $V_1$ | — | tensão 1 | tensão chamada $V_1$ no circuito | V | [amp-diff](exemplos/amp-diff.md) |
| $v_1$ | — | tensão instantânea 1 | tensão instantânea chamada $v_1$ no circuito | V | [chua](exemplos/chua.md) |
| $V_2$ | — | tensão 2 | tensão chamada $V_2$ no circuito | V | [amp-diff](exemplos/amp-diff.md) |
| $v_2$ | — | tensão instantânea 2 | tensão instantânea chamada $v_2$ no circuito | V | [chua](exemplos/chua.md) |
| $V_\infty$ | — | tensão final | valor para o qual a tensão tende depois do transitório | V | [relayosc](exemplos/relayosc.md) |
| $V_i$ | — | tensão de entrada | tensão da entrada do circuito | V | [freqdouble](exemplos/freqdouble.md) |
| $v_i$ | — | tensão instantânea de entrada | tensão da entrada em cada instante | V | [pushpull](exemplos/pushpull.md) |
| $V_{\text{in}}$ | — | tensão de entrada | sinal aplicado à entrada do circuito | V | [allpass1](exemplos/allpass1.md) |
| $v_{\text{in}}$ | — | tensão instantânea de entrada | sinal aplicado à entrada do circuito | V | [ccdiff](exemplos/ccdiff.md) |
| $V_n$ | — | tensão nominal | tensão para a qual a lâmpada é especificada | V | [lightbulb](exemplos/lightbulb.md) |
| $V_o$ | — | tensão de saída | tensão da saída do circuito | V | [howland](exemplos/howland.md) |
| $v_o$ | — | tensão instantânea de saída | tensão da saída em cada instante | V | [pushpull](exemplos/pushpull.md) |
| $V_{\text{out}}$ | — | tensão de saída | sinal que o circuito entrega à saída | V | [allpass1](exemplos/allpass1.md) |
| $v_{\text{out}}$ | — | tensão instantânea de saída | sinal que o circuito entrega à saída | V | [diff](exemplos/diff.md) |
| $v_{R,\text{pico}}$ | — | tensão de pico no resistor | maior valor atingido pela tensão no resistor | V | [fullrect-sc](exemplos/fullrect-sc.md) |
| $V_{\text{ref}}$ | — | tensão de referência | tensão fixa com que a entrada é comparada | V | [diodeclip](exemplos/diodeclip.md) |
| $V_{\text{saida}}$ | — | tensão de saída | tensão que o circuito entrega à saída | V | [diodeclip](exemplos/diodeclip.md) |
| $v_{\text{saida}}$ | — | tensão instantânea de saída | tensão da saída em cada instante | V | [dcrestoration](exemplos/dcrestoration.md) |
| $V_{\text{saida},\text{max}}$ | — | tensão de saída máxima | maior valor que a tensão de saída atinge | V | [diodeclip](exemplos/diodeclip.md) |
| $V_{\text{saída}}$ | — | tensão de saída | tensão que o circuito entrega à saída | V | [ccitov](exemplos/ccitov.md) |
| $V_{\text{th}}$ | — | tensão de Thévenin | tensão em aberto entre os terminais do circuito | V | [norton](exemplos/norton.md) |

## Tempo e frequência

| Símbolo | Grafias | Nome | Significado | Unidade | Exemplo |
|---------|---------|------|-------------|---------|---------|
| $f$ | — | frequência | número de ciclos por segundo de um sinal periódico | Hz | [555int](exemplos/555int.md) |
| $f_1$ | — | frequência 1 | frequência chamada $f_1$ no circuito | Hz | [butterbandstop](exemplos/butterbandstop.md) |
| $f_2$ | — | frequência 2 | frequência chamada $f_2$ no circuito | Hz | [butterbandstop](exemplos/butterbandstop.md) |
| $f_a$ | — | frequência do modo a | frequência do modo em fase | Hz | [coupled2](exemplos/coupled2.md) |
| $f_b$ | — | frequência do modo b | frequência do modo em oposição | Hz | [coupled2](exemplos/coupled2.md) |
| $f_{\text{bat}}$ | — | frequência de batimento | diferença entre as duas frequências, $f_b - f_a$ | Hz | [coupled2](exemplos/coupled2.md) |
| $f_{\text{clk}}$ | — | frequência do relógio | frequência com que as chaves são acionadas | Hz | [counter8](exemplos/counter8.md) |
| $f_{\text{ent}}$ | — | frequência de entrada | frequência do sinal de entrada | Hz | [freqdouble](exemplos/freqdouble.md) |
| $f_k$ | — | frequência do estágio k | frequência característica do estágio de índice $k$ | Hz | [phaseseq](exemplos/phaseseq.md) |
| $f_{\text{max}}$ | — | frequência máxima | maior frequência da varredura | Hz | [bandpass](exemplos/bandpass.md) |
| $f_{\text{min}}$ | — | frequência mínima | menor frequência da varredura | Hz | [bandpass](exemplos/bandpass.md) |
| $f_{\text{out}}$ | — | frequência de saída | frequência do sinal que o circuito entrega | Hz | [digsine](exemplos/digsine.md) |
| $f_{\text{rel}}$ | — | frequência do relógio | frequência do sinal de relógio | Hz | [relayctr](exemplos/relayctr.md) |
| $f_{VCO}$ | — | frequência do oscilador controlado | frequência de saída do oscilador controlado por tensão | Hz | [freqdouble](exemplos/freqdouble.md) |
| $f_x$ | — | frequência em x | frequência do sinal aplicado ao eixo $x$ | Hz | [lissa](exemplos/lissa.md) |
| $f_y$ | — | frequência em y | frequência do sinal aplicado ao eixo $y$ | Hz | [lissa](exemplos/lissa.md) |
| $n$ | — | índice do harmônico | ordem do harmônico de um sinal periódico (1 é a fundamental) | — | [sinediode](exemplos/sinediode.md) |
| $T$ | — | período | duração de um ciclo completo de um sinal periódico | s | [555dutycycle](exemplos/555dutycycle.md) |
| $t$ | — | tempo | instante, contado a partir do início da simulação | s | [3phasewye](exemplos/3phasewye.md) |
| $t_0$ | — | instante de referência | instante em que o evento descrito começa | s | [555sequencer](exemplos/555sequencer.md) |
| $T_{\text{bat}}$ | — | período de batimento | $1/f_\text{bat}$: período do batimento entre duas frequências próximas | s | [coupled2](exemplos/coupled2.md) |
| $V_1$ | — | amplitude da fundamental | amplitude do primeiro harmônico da saída | V | [sinediode](exemplos/sinediode.md) |
| $V_3$ | — | amplitude do 3º harmônico | amplitude do harmônico de ordem 3 da saída | V | [sinediode](exemplos/sinediode.md) |
| $V_5$ | — | amplitude do 5º harmônico | amplitude do harmônico de ordem 5 da saída | V | [sinediode](exemplos/sinediode.md) |
| $V_7$ | — | amplitude do 7º harmônico | amplitude do harmônico de ordem 7 da saída | V | [sinediode](exemplos/sinediode.md) |
| $V_n$ | — | amplitude do harmônico n | amplitude do harmônico de ordem $n$ (para a triangular, $V_1/n^2$) | V | [sinediode](exemplos/sinediode.md) |
| $\omega$ | — | frequência angular | $2\pi f$: a frequência medida em radianos por segundo | rad/s | [3motor](exemplos/3motor.md) |
| $\omega_a$ | — | frequência angular do modo a | frequência angular do modo em fase | rad/s | [coupled2](exemplos/coupled2.md) |
| $\omega_b$ | — | frequência angular do modo b | frequência angular do modo em oposição | rad/s | [coupled2](exemplos/coupled2.md) |
| $\omega_c$ | — | frequência angular da portadora | $2\pi f_c$: frequência da portadora em radianos por segundo | rad/s | [amdetect](exemplos/amdetect.md) |
| $\omega_k$ | — | frequência angular do modo k | frequência angular natural do modo de índice $k$ | rad/s | [coupled1](exemplos/coupled1.md) |
| $\omega_m$ | — | frequência angular da modulante | $2\pi f_m$: frequência angular do sinal modulante | rad/s | [ota-ringmod](exemplos/ota-ringmod.md) |
| $\phi$ | — | fase | ângulo de defasagem de um sinal em relação a uma referência | rad | [varactor](exemplos/varactor.md) |
| $\tau$ | — | constante de tempo | tempo característico de carga ou descarga de um circuito (por exemplo, $RC$ ou $L/R$) | s | [allpass2](exemplos/allpass2.md) |
| $\tau_1$ | — | constante de tempo lenta | constante de tempo da parte lenta do transitório | s | [transformerdc](exemplos/transformerdc.md) |
| $\tau_2$ | — | constante de tempo rápida | constante de tempo da parte rápida do transitório | s | [transformerdc](exemplos/transformerdc.md) |
| $\tau_b$ | — | constante de tempo da bobina | $L/R_b$: tempo com que a corrente da bobina cresce | s | [relay](exemplos/relay.md) |
| $\tau_c$ | — | constante de tempo de carga | constante de tempo com que o capacitor se carrega | s | [relayosc](exemplos/relayosc.md) |
| $\tau_d$ | — | constante de tempo de descarga | constante de tempo com que o capacitor se descarrega | s | [relayosc](exemplos/relayosc.md) |
| $\theta$ | — | ângulo | ângulo de fase ou de condução | rad | [tlfreq](exemplos/tlfreq.md) |
| $\varphi$ | — | fase | ângulo de defasagem de um sinal em relação a uma referência | rad | [allpass1](exemplos/allpass1.md) |

## Componentes passivos

| Símbolo | Grafias | Nome | Significado | Unidade | Exemplo |
|---------|---------|------|-------------|---------|---------|
| $a$ | — | nó a | ponto de ligação identificado como $a$ no circuito | — | [cs-currentadder](exemplos/cs-currentadder.md) |
| $a_{k,m}$ | — | amplitude do modo m na malha k | amplitude da corrente da malha $k$ no modo $m$ | A | [coupled3](exemplos/coupled3.md) |
| $B$ | — | densidade de fluxo magnético | campo magnético $B$ no núcleo, que satura com a corrente | T | [satcore-inductor](exemplos/satcore-inductor.md) |
| $B$ | — | susceptância | parte imaginária da admitância | S | [tllopass](exemplos/tllopass.md) |
| $b$ | — | nó b | o outro nó entre os quais se mede a resistência equivalente | — | [cs-currentadder](exemplos/cs-currentadder.md) |
| $C$ | — | capacitância | quanta carga um capacitor guarda por volt aplicado | F | [555dutycycle](exemplos/555dutycycle.md) |
| $C_0$ | — | capacitância paralela do cristal | capacitância $C_0$ dos eletrodos, em paralelo com o ramo em série | F | [crystalosc](exemplos/crystalosc.md) |
| $C_1$ | — | capacitância do capacitor 1 | capacitância do capacitor chamado $C_1$ no circuito | F | [actbutterhi](exemplos/actbutterhi.md) |
| $C_{15}$ | — | capacitância do capacitor 15 | capacitância do capacitor chamado $C_{15}$ no circuito | F | [sine](exemplos/sine.md) |
| $C_2$ | — | capacitância do capacitor 2 | capacitância do capacitor chamado $C_2$ no circuito | F | [actbutterhi](exemplos/actbutterhi.md) |
| $C_3$ | — | capacitância do capacitor 3 | capacitância do capacitor chamado $C_3$ no circuito | F | [twint](exemplos/twint.md) |
| $C_{\text{eq}}$ | — | capacitância equivalente | capacitância de um conjunto de capacitores visto como um só | F | [capmult](exemplos/capmult.md) |
| $C_k$ | — | capacitância do estágio k | capacitor de índice $k$ do conjunto | F | [cappar](exemplos/cappar.md) |
| $C_L$ | — | capacitância de carga | capacitor ligado à saída, que o circuito precisa carregar | F | [cmosinverterslow](exemplos/cmosinverterslow.md) |
| $C_s$ | — | capacitância em série do cristal | capacitância $C_s$ do ramo ressonante em série do cristal | F | [crystalosc](exemplos/crystalosc.md) |
| $C_{\text{tot}}$ | — | capacitância total | capacitância total do circuito ressonante | F | [varactorvco](exemplos/varactorvco.md) |
| $G$ | — | condutância | inverso da resistência, $1/R$ | S | [dram](exemplos/dram.md) |
| $G$ | — | matriz de condutâncias | matriz $G$ do sistema nodal de uma rede de resistores | S | [grid](exemplos/grid.md) |
| $G_{\text{ii}}$ | — | condutância própria do nó i | soma das condutâncias que chegam ao nó $i$ | S | [grid](exemplos/grid.md) |
| $G_{\text{ij}}$ | — | condutância entre os nós i e j | condutância $1/R_{ij}$ do ramo que liga os nós $i$ e $j$ | S | [grid](exemplos/grid.md) |
| $G_j$ | — | condutância do ramo j | condutância do ramo de índice $j$ de um divisor de corrente | S | [resistors](exemplos/resistors.md) |
| $G_k$ | — | condutância do ramo k | condutância do ramo de índice $k$ de um divisor de corrente | S | [resistors](exemplos/resistors.md) |
| $I_a$ | — | corrente no trecho a | corrente no trecho $a$ do potenciômetro | A | [pot](exemplos/pot.md) |
| $I_b$ | — | corrente no trecho b | corrente no trecho $b$ do potenciômetro | A | [pot](exemplos/pot.md) |
| $i_b$ | — | corrente da bobina | corrente que passa pela bobina do relé | A | [3motor](exemplos/3motor.md) |
| $I_g$ | — | corrente no medidor | corrente que passa pelo medidor ligado à ponte | A | [wheatstone](exemplos/wheatstone.md) |
| $i_L$ | — | corrente no indutor | corrente que passa pelo indutor | A | [chua](exemplos/chua.md) |
| $I_{\text{off}}$ | — | corrente de soltura | corrente da bobina abaixo da qual o relé solta | A | [relay](exemplos/relay.md) |
| $I_{\text{on}}$ | — | corrente de operação | corrente da bobina acima da qual o relé opera | A | [relay](exemplos/relay.md) |
| $I_{\text{sat}}$ | — | corrente de saturação do núcleo | corrente em que o núcleo do indutor satura | A | [satcore-comparison](exemplos/satcore-comparison.md) |
| $k$ | — | coeficiente de acoplamento | fração do fluxo de uma bobina que atravessa a outra (de $0$ a $1$) | — | [3motor](exemplos/3motor.md) |
| $L$ | — | indutância | quanto fluxo magnético uma bobina cria por ampère | H | [3way](exemplos/3way.md) |
| $L$ | — | indutância do cristal | indutância $L$ do ramo ressonante em série do cristal | H | [crystalosc](exemplos/crystalosc.md) |
| $L_0$ | — | indutância sem saturação | indutância do indutor com corrente pequena | H | [satcore-comparison](exemplos/satcore-comparison.md) |
| $L_1$ | — | indutância do indutor 1 | indutância do indutor chamado $L_1$ no circuito | H | [conv-cuk](exemplos/conv-cuk.md) |
| $L_2$ | — | indutância do indutor 2 | indutância do indutor chamado $L_2$ no circuito | H | [conv-cuk](exemplos/conv-cuk.md) |
| $L_\sigma$ | — | indutância de dispersão | parte da indutância que não se acopla ao outro enrolamento | H | [3motor](exemplos/3motor.md) |
| $L_{\text{eq}}$ | — | indutância equivalente | indutância de um conjunto de indutores visto como um só | H | [indpar](exemplos/indpar.md) |
| $L_k$ | — | indutância do indutor k | indutância do indutor de índice $k$ | H | [indpar](exemplos/indpar.md) |
| $L_s$ | — | indutância do estator | indutância do enrolamento do estator | H | [3motor](exemplos/3motor.md) |
| $M$ | — | indutância mútua | acoplamento entre duas bobinas: o fluxo de uma induz tensão na outra | H | [indseries](exemplos/indseries.md) |
| $N$ | — | número de malhas | quantidade de malhas LC acopladas | — | [coupled1](exemplos/coupled1.md) |
| $N$ | — | número de seções | quantidade de seções iguais da escada | — | [ladder](exemplos/ladder.md) |
| $n$ | — | razão de espiras | $N_1/N_2$: relação entre o número de espiras do primário e do secundário | — | [longdist](exemplos/longdist.md) |
| $N_1$ | — | espiras do primário | número de espiras do enrolamento primário do transformador | — | [joule-thief](exemplos/joule-thief.md) |
| $N_2$ | — | espiras do secundário | número de espiras do enrolamento secundário do transformador | — | [joule-thief](exemplos/joule-thief.md) |
| $Q$ | — | carga elétrica | quantidade de carga acumulada | C | [capseries](exemplos/capseries.md) |
| $q_1$ | — | carga da malha 1 | carga que passou pela malha 1 | C | [coupled1](exemplos/coupled1.md) |
| $q_2$ | — | carga da malha 2 | carga que passou pela malha 2 | C | [coupled1](exemplos/coupled1.md) |
| $q_k$ | — | carga da malha k | carga que passou pela malha de índice $k$ | C | [coupled1](exemplos/coupled1.md) |
| $R$ | — | resistência | oposição de um resistor à passagem da corrente ($V = R I$) | Ω | [3phasewye](exemplos/3phasewye.md) |
| $R_1$ | — | resistência do resistor 1 | resistência do resistor chamado $R_1$ no circuito | Ω | [actbutterhi](exemplos/actbutterhi.md) |
| $R_{10}$ | — | resistência do resistor 10 | resistência do resistor chamado $R_{10}$ no circuito | Ω | [opamp-regulator](exemplos/opamp-regulator.md) |
| $R_{11}$ | — | resistência do resistor 11 | resistência do resistor chamado $R_{11}$ no circuito | Ω | [nic-r](exemplos/nic-r.md) |
| $R_{13}$ | — | resistência do resistor 13 | resistência do resistor chamado $R_{13}$ no circuito | Ω | [sine](exemplos/sine.md) |
| $R_{17}$ | — | resistência do resistor 17 | resistência do resistor chamado $R_{17}$ no circuito | Ω | [sine](exemplos/sine.md) |
| $R_2$ | — | resistência do resistor 2 | resistência do resistor chamado $R_2$ no circuito | Ω | [actbutterhi](exemplos/actbutterhi.md) |
| $R_3$ | — | resistência do resistor 3 | resistência do resistor chamado $R_3$ no circuito | Ω | [amp-diff](exemplos/amp-diff.md) |
| $R_4$ | — | resistência do resistor 4 | resistência do resistor chamado $R_4$ no circuito | Ω | [amp-diff](exemplos/amp-diff.md) |
| $R_5$ | — | resistência do resistor 5 | resistência do resistor chamado $R_5$ no circuito | Ω | [nic-r](exemplos/nic-r.md) |
| $R_7$ | — | resistência do resistor 7 | resistência do resistor chamado $R_7$ no circuito | Ω | [relaxosc](exemplos/relaxosc.md) |
| $R_9$ | — | resistência do resistor 9 | resistência do resistor chamado $R_9$ no circuito | Ω | [opamp-regulator](exemplos/opamp-regulator.md) |
| $R_a$ | — | resistência do trecho a | resistência entre uma extremidade e o cursor do potenciômetro | Ω | [555dutycycle](exemplos/555dutycycle.md) |
| $R_{\text{ab}}$ | — | resistência entre os nós a e b | resistência equivalente vista entre os nós $a$ e $b$ | Ω | [grid](exemplos/grid.md) |
| $R_{\text{baixo}}$ | — | resistência de baixo | resistência da parte de baixo da associação | Ω | [currentsrcelm](exemplos/currentsrcelm.md) |
| $R_{\text{cima}}$ | — | resistência de cima | resistência da parte de cima da associação | Ω | [currentsrcelm](exemplos/currentsrcelm.md) |
| $R_{\text{contato}}$ | — | resistência de contato | resistência do contato fechado | Ω | [latchingrelay](exemplos/latchingrelay.md) |
| $R_{\text{eq}}$ | — | resistência equivalente | resistência de um conjunto de resistores visto como um só | Ω | [cube](exemplos/cube.md) |
| $R_g$ | — | resistência do medidor | resistência interna do medidor ligado à ponte | Ω | [wheatstone](exemplos/wheatstone.md) |
| $R_{\text{ij}}$ | — | resistência entre os nós i e j | resistência do ramo que liga os nós $i$ e $j$ | Ω | [grid](exemplos/grid.md) |
| $R_{\text{in}}$ | — | resistência de entrada | resistência vista pela fonte olhando para a entrada do circuito | Ω | [norton-invert](exemplos/norton-invert.md) |
| $R_k$ | — | resistência do resistor k | resistência do resistor de índice $k$ | Ω | [currentsrcelm](exemplos/currentsrcelm.md) |
| $R_L$ | — | resistência de carga | resistor ligado à saída, que consome a energia do circuito | Ω | [amp-follower](exemplos/amp-follower.md) |
| $R_l$ | — | resistência da linha | resistência dos fios da linha de transmissão | Ω | [powerfactor1](exemplos/powerfactor1.md) |
| $R_{\text{lamp}}$ | — | resistência da lâmpada | resistência do filamento da lâmpada | Ω | [wienbridge](exemplos/wienbridge.md) |
| $R_{\text{linha}}$ | — | resistência da linha | resistência dos fios da linha de transmissão | Ω | [longdist](exemplos/longdist.md) |
| $R_N$ | — | resistência de Norton | resistência vista dos terminais com as fontes desligadas | Ω | [norton](exemplos/norton.md) |
| $R_n$ | — | resistência do resistor n | resistência do último resistor do conjunto | Ω | [voltdivide](exemplos/voltdivide.md) |
| $R_{\text{nom}}$ | — | resistência nominal | resistência do filamento na temperatura de operação | Ω | [lightbulb](exemplos/lightbulb.md) |
| $R_{\text{off}}$ | — | resistência de corte | resistência de uma chave ou de um transistor quando não conduz | Ω | [mr](exemplos/mr.md) |
| $R_{\text{on}}$ | — | resistência de condução | resistência de uma chave ou de um transistor quando conduz | Ω | [cmostransgate](exemplos/cmostransgate.md) |
| $R_{\text{out}}$ | — | resistência de saída | resistência vista pela carga olhando para a saída do circuito | Ω | [amp-follower](exemplos/amp-follower.md) |
| $R_p$ | — | resistência em paralelo | resistência equivalente de resistores ligados em paralelo | Ω | [res-par](exemplos/res-par.md) |
| $R_r$ | — | resistência do rotor | resistência do enrolamento do rotor | Ω | [3motor](exemplos/3motor.md) |
| $R_{\text{rede}}$ | — | resistência da rede | resistência equivalente de uma rede de resistores em paralelo | Ω | [currentsrcelm](exemplos/currentsrcelm.md) |
| $R_{\text{resto}}$ | — | resistência do resto da rede | resistência equivalente do restante da rede | Ω | [grid2](exemplos/grid2.md) |
| $R_s$ | — | resistência em série | resistor em série com a fonte ou com o dispositivo | Ω | [3motor](exemplos/3motor.md) |
| $r_{\text{sai}}$ | — | resistência de saída | resistência incremental vista pela carga | Ω | [zenerreffollow](exemplos/zenerreffollow.md) |
| $R_t$ | — | resistência do bloco superior | resistência do bloco de cima de um divisor de tensão | Ω | [resistors](exemplos/resistors.md) |
| $R_{\text{th}}$ | — | resistência de Thévenin | resistência vista dos terminais com as fontes desligadas | Ω | [norton](exemplos/norton.md) |
| $V_b$ | — | tensão no bloco inferior | tensão sobre o bloco de baixo de um divisor de tensão | V | [resistors](exemplos/resistors.md) |
| $V_C$ | — | tensão no capacitor | tensão entre os terminais do capacitor | V | [capmult](exemplos/capmult.md) |
| $v_C$ | — | tensão no capacitor | tensão entre os terminais do capacitor | V | [3phasewye](exemplos/3phasewye.md) |
| $V_{C,\text{pico}}$ | — | tensão de pico no capacitor | valor máximo da tensão sobre o capacitor | V | [capac](exemplos/capac.md) |
| $V_k$ | — | tensão no resistor k | tensão sobre o resistor de índice $k$ | V | [voltdivide](exemplos/voltdivide.md) |
| $V_L$ | — | tensão no indutor | tensão entre os terminais do indutor | V | [indmultfreq](exemplos/indmultfreq.md) |
| $v_L$ | — | tensão no indutor | tensão entre os terminais do indutor | V | [indpar](exemplos/indpar.md) |
| $V_{L,\text{pico}}$ | — | tensão de pico no indutor | valor máximo da tensão sobre o indutor | V | [indmultfreq](exemplos/indmultfreq.md) |
| $V_R$ | — | tensão no resistor | tensão sobre o resistor | V | [varactor](exemplos/varactor.md) |
| $v_R$ | — | tensão no resistor | tensão entre os terminais do resistor | V | [diff](exemplos/diff.md) |
| $X$ | — | reatância | parte imaginária da impedância | Ω | [impedance](exemplos/impedance.md) |
| $x$ | — | posição do cursor | fração do curso do potenciômetro, de $0$ a $1$ | — | [pot](exemplos/pot.md) |
| $X_C$ | — | reatância capacitiva | oposição de um capacitor à corrente alternada, $1/(\omega C)$ | Ω | [capac](exemplos/capac.md) |
| $X_L$ | — | reatância indutiva | oposição de um indutor à corrente alternada, $\omega L$ | Ω | [capac](exemplos/capac.md) |
| $Y$ | — | admitância | inverso da impedância | S | [bandpass](exemplos/bandpass.md) |
| $Y_{\text{in}}$ | — | admitância de entrada | inverso da impedância de entrada, $I/V$ | S | [capmult](exemplos/capmult.md) |
| $Z$ | — | impedância | oposição de um elemento à corrente alternada, na frequência considerada | Ω | [capac](exemplos/capac.md) |
| $Z_1$ | — | impedância 1 | impedância chamada $Z_1$ no circuito | Ω | [longdist](exemplos/longdist.md) |
| $Z_2$ | — | impedância 2 | impedância chamada $Z_2$ no circuito | Ω | [longdist](exemplos/longdist.md) |
| $Z_C$ | — | impedância do capacitor | $1/(j\omega C)$ | Ω | [capac](exemplos/capac.md) |
| $Z_{\text{in}}$ | — | impedância de entrada | impedância vista pela fonte olhando para a entrada do circuito | Ω | [ccinductor](exemplos/ccinductor.md) |
| $Z_L$ | — | impedância de carga | impedância ligada ao fim da linha ou à saída | Ω | [gyratorelm](exemplos/gyratorelm.md) |
| $Z_L$ | — | impedância do indutor | $j\omega L$ | Ω | [capac](exemplos/capac.md) |
| $Z_R$ | — | impedância do resistor | $R$ | Ω | [capac](exemplos/capac.md) |
| $Z_{RC}$ | — | impedância do conjunto RC | impedância de um resistor em série com um capacitor | Ω | [impedance](exemplos/impedance.md) |
| $Z_{\text{refletida}}$ | — | impedância refletida | impedância da carga vista do primário, $n^2 R_L$ | Ω | [longdist](exemplos/longdist.md) |
| $Z_{RL}$ | — | impedância do conjunto RL | impedância de um resistor em série com um indutor | Ω | [impedance](exemplos/impedance.md) |

## Fontes e formas de onda

| Símbolo | Grafias | Nome | Significado | Unidade | Exemplo |
|---------|---------|------|-------------|---------|---------|
| $A$ | — | amplitude da portadora | amplitude da portadora sem modulação | V | [amdetect](exemplos/amdetect.md) |
| $A$ | — | amplitude da senoide | valor de pico de uma senoide | — | [phaseseq](exemplos/phaseseq.md) |
| $c$ | — | portadora | onda quadrada $c(t)$ de $\pm 1$ que liga e desliga os diodos do anel | — | [ringmod](exemplos/ringmod.md) |
| $f_c$ | — | frequência da portadora | frequência da portadora que aciona o anel de diodos | Hz | [ringmod](exemplos/ringmod.md) |
| $f_s$ | — | frequência do sinal | frequência do sinal de entrada $s(t)$ | Hz | [ringmod](exemplos/ringmod.md) |
| $i_A$ | — | corrente da fase A | corrente da fase A de um sistema trifásico | A | [3phasewye](exemplos/3phasewye.md) |
| $i_B$ | — | corrente da fase B | corrente da fase B de um sistema trifásico | A | [3phasewye](exemplos/3phasewye.md) |
| $i_C$ | — | corrente da fase C | corrente da fase C de um sistema trifásico | A | [3phasewye](exemplos/3phasewye.md) |
| $I_{\text{ef}}$ | — | corrente eficaz | valor eficaz da corrente | A | [lightbulb](exemplos/lightbulb.md) |
| $i_k$ | — | corrente da fase k | corrente da fase de índice $k$ | A | [3phasewye](exemplos/3phasewye.md) |
| $i_N$ | — | corrente do neutro | soma das correntes das três fases | A | [3phasewye](exemplos/3phasewye.md) |
| $I_p$ | — | corrente de pico | valor máximo da corrente | A | [3way](exemplos/3way.md) |
| $k$ | — | índice da banda lateral | inteiro $0, 1, 2, \dots$ que numera as bandas laterais ímpares | — | [ringmod](exemplos/ringmod.md) |
| $m$ | — | índice de modulação | quanto a amplitude da portadora varia com o sinal | — | [amdetect](exemplos/amdetect.md) |
| $s$ | — | sinal de entrada | sinal $s(t)$ que o modulador em anel multiplica pela portadora | V | [ringmod](exemplos/ringmod.md) |
| $s$ | — | sinal modulante | sinal de informação $s(t)$ que modula a amplitude da portadora | — | [amdetect](exemplos/amdetect.md) |
| $u$ | — | tensão de entrada | sinal $u(t)$ aplicado à entrada do circuito | V | [ringing](exemplos/ringing.md) |
| $v_A$ | — | tensão da fase A | tensão da fase A de um sistema trifásico | V | [3phasewye](exemplos/3phasewye.md) |
| $v_a$ | — | tensão instantânea do áudio | sinal de áudio a amplificar, em cada instante | V | [classd](exemplos/classd.md) |
| $V_{\text{ef}}$ | — | tensão eficaz | valor eficaz (RMS) da tensão | V | [3phasewye](exemplos/3phasewye.md) |
| $v_k$ | — | tensão da fase k | tensão da fase de índice $k$ | V | [3phasewye](exemplos/3phasewye.md) |
| $V_{\text{linha}}$ | — | tensão de linha | tensão entre duas fases de um sistema trifásico | V | [3phasewye](exemplos/3phasewye.md) |
| $V_{\text{med}}$ | — | tensão média | valor médio da tensão ao longo de um ciclo | V | [fullrect](exemplos/fullrect.md) |
| $V_{\text{out},\text{pico}}$ | — | tensão de pico na saída | valor máximo da tensão de saída | V | [bandpass](exemplos/bandpass.md) |
| $V_p$ | — | tensão de pico | valor máximo atingido por uma senoide | V | [3motor](exemplos/3motor.md) |
| $V_{\text{pico}}$ | — | tensão de pico | valor máximo atingido pela tensão | V | [capac](exemplos/capac.md) |
| $V_{\text{rede}}$ | — | tensão da rede | tensão eficaz da rede elétrica | V | [triacdimmer](exemplos/triacdimmer.md) |
| $V_{\text{rms}}$ | — | tensão eficaz | valor eficaz (RMS): a raiz da média do quadrado da tensão em um período | V | [cs-fullrectifier](exemplos/cs-fullrectifier.md) |
| $V_{\text{rms},\text{in}}$ | — | tensão eficaz de entrada | valor eficaz da tensão de entrada | V | [actbutterband](exemplos/actbutterband.md) |
| $V_{\text{rms},\text{out}}$ | — | tensão eficaz de saída | valor eficaz da tensão de saída | V | [actbutterband](exemplos/actbutterband.md) |
| $V_s$ | — | tensão da fonte | tensão da fonte de alimentação alternada | V | [scractrig](exemplos/scractrig.md) |
| $v_s$ | — | tensão instantânea da fonte | tensão do sinal da fonte de entrada em cada instante | V | [diodelimit](exemplos/diodelimit.md) |
| $\omega_s$ | — | frequência angular do sinal | $2\pi f_s$: frequência do sinal de entrada em radianos por segundo | rad/s | [ringmod](exemplos/ringmod.md) |

## Diodos

| Símbolo | Grafias | Nome | Significado | Unidade | Exemplo |
|---------|---------|------|-------------|---------|---------|
| $I_{LED}$ | — | corrente no LED | corrente que passa pelo LED | A | [joule-thief](exemplos/joule-thief.md) |
| $I_P$ | — | corrente de pico | corrente no pico da curva do dispositivo | A | [tdiode](exemplos/tdiode.md) |
| $I_{\text{rev}}$ | — | corrente reversa | corrente que passa pelo diodo bloqueado | A | [peak-detect](exemplos/peak-detect.md) |
| $I_S$ | — | corrente de saturação | corrente reversa de um diodo ou de uma junção | A | [logconvert](exemplos/logconvert.md) |
| $I_s$ | `Is` | corrente de saturação | corrente reversa de um diodo ou de uma junção | A | [amdetect](exemplos/amdetect.md) |
| $I_V$ | — | corrente de vale | corrente no vale da curva do dispositivo | A | [tdiode](exemplos/tdiode.md) |
| $i_Z$ | — | corrente instantânea no zener | corrente que passa pelo diodo zener em cada instante | A | [zenerref](exemplos/zenerref.md) |
| $k$ | — | constante de Boltzmann | $1{,}381\cdot 10^{-23}\ \text{J/K}$, que liga temperatura e energia | J/K | [logconvert](exemplos/logconvert.md) |
| $n$ | — | fator de idealidade | expoente do diodo na equação de Shockley, $i = I_s(e^{v/(nV_T)}-1)$ | — | [amdetect](exemplos/amdetect.md) |
| $q$ | — | carga do elétron | carga elétrica elementar, $1{,}602\cdot 10^{-19}\ \text{C}$ | C | [lissa](exemplos/lissa.md) |
| $r_d$ | — | resistência dinâmica do diodo | inclinação $dv/di$ da curva do diodo no ponto de operação | Ω | [diodevar](exemplos/diodevar.md) |
| $r_Z$ | — | resistência dinâmica do zener | inclinação $dv/di$ da curva do zener na região de ruptura | Ω | [zeneriv](exemplos/zeneriv.md) |
| $v_0$ | — | parâmetro de ruptura do zener | parâmetro $v_0$ do modelo, que desloca o início da ruptura inversa do zener | V | [zeneriv](exemplos/zeneriv.md) |
| $V_{\text{alta}}$ | — | tensão do ramo alto | tensão do ramo de tensão alta da curva | V | [tdrelax](exemplos/tdrelax.md) |
| $V_{\text{baixa}}$ | — | tensão do ramo baixo | tensão do ramo de tensão baixa da curva | V | [tdrelax](exemplos/tdrelax.md) |
| $V_D$ | — | tensão no diodo | tensão entre o ânodo e o cátodo do diodo | V | [amp-fullrect](exemplos/amp-fullrect.md) |
| $v_D$ | — | tensão instantânea no diodo | tensão entre o ânodo e o cátodo do diodo em cada instante | V | [diodeclip](exemplos/diodeclip.md) |
| $V_{LED}$ | — | tensão no LED | tensão entre os terminais do LED aceso | V | [joule-thief](exemplos/joule-thief.md) |
| $V_P$ | — | tensão de pico do dispositivo | tensão em que o dispositivo passa a conduzir (pico da curva) | V | [tdiode](exemplos/tdiode.md) |
| $V_{PP}$ | — | tensão de pico do diodo | tensão do pico da curva do diodo túnel | V | [tdiode](exemplos/tdiode.md) |
| $V_T$ | — | tensão térmica | $kT/q$: cerca de $25{,}85\ \text{mV}$ a $300\ \text{K}$ | V | [amdetect](exemplos/amdetect.md) |
| $V_V$ | — | tensão de vale | tensão do vale da curva do dispositivo | V | [spark-sawtooth](exemplos/spark-sawtooth.md) |
| $V_Z$ | — | tensão zener | tensão em que o diodo zener regula | V | [cc2](exemplos/cc2.md) |

## Transistores bipolares

| Símbolo | Grafias | Nome | Significado | Unidade | Exemplo |
|---------|---------|------|-------------|---------|---------|
| $A_{\text{cm}}$ | — | ganho de modo comum | razão entre a saída e a tensão de modo comum de entrada | — | [trans-diffamp](exemplos/trans-diffamp.md) |
| $A_d$ | — | ganho diferencial | razão entre a saída e a tensão diferencial de entrada | — | [trans-diffamp](exemplos/trans-diffamp.md) |
| $g_o$ | — | condutância de saída | inclinação $\partial I_C/\partial V_{CE}$ das curvas de saída do transistor bipolar | S | [early](exemplos/early.md) |
| $I_B$ | `Ib` | corrente de base | corrente que entra pela base do transistor bipolar | A | [ceamp](exemplos/ceamp.md) |
| $I_{B1}$ | — | corrente de base 1 | corrente que entra pela base do transistor bipolar (do elemento de número 1) | A | [darlington](exemplos/darlington.md) |
| $I_C$ | `Ic` | corrente de coletor | corrente que entra pelo coletor do transistor bipolar | A | [ceamp](exemplos/ceamp.md) |
| $i_c$ | — | corrente de coletor de pequeno sinal | variação da corrente do coletor em torno do ponto de operação | A | [3motor](exemplos/3motor.md) |
| $I_{C0}$ | — | corrente de coletor sem efeito Early | valor de $I_C$ que se teria sem a inclinação causada pelo efeito Early | A | [early](exemplos/early.md) |
| $I_{C1}$ | — | corrente de coletor 1 | corrente que entra pelo coletor do transistor bipolar (do elemento de número 1) | A | [mirror](exemplos/mirror.md) |
| $I_{C2}$ | — | corrente de coletor 2 | corrente que entra pelo coletor do transistor bipolar (do elemento de número 2) | A | [mirror](exemplos/mirror.md) |
| $I_E$ | `Ie` | corrente de emissor | corrente que sai pelo emissor do transistor bipolar | A | [ceamp](exemplos/ceamp.md) |
| $I_{E1}$ | — | corrente de emissor 1 | corrente que sai pelo emissor do transistor bipolar (do elemento de número 1) | A | [darlington](exemplos/darlington.md) |
| $I_{E3}$ | — | corrente de emissor 3 | corrente que sai pelo emissor do transistor bipolar (do elemento de número 3) | A | [trans-diffamp-cursrc](exemplos/trans-diffamp-cursrc.md) |
| $I_{EE}$ | — | corrente da cauda | corrente da fonte comum aos emissores do par diferencial | A | [gilbertcell](exemplos/gilbertcell.md) |
| $I_{\text{pol}}$ | — | corrente de polarização | corrente contínua que mantém os transistores levemente ligados | A | [pushpull](exemplos/pushpull.md) |
| $I_{\text{ref}}$ | — | corrente de referência | corrente de entrada que o espelho de corrente copia | A | [deltasigma](exemplos/deltasigma.md) |
| $I_T$ | — | corrente da cauda | corrente total que o par diferencial tira da cauda | A | [trans-diffamp](exemplos/trans-diffamp.md) |
| $I_x$ | — | corrente diferencial do canal X | diferença entre as correntes de coletor do par diferencial do canal X | A | [gilbertcell](exemplos/gilbertcell.md) |
| $I_y$ | — | corrente diferencial do canal Y | diferença entre as correntes de coletor do par diferencial do canal Y | A | [gilbertcell](exemplos/gilbertcell.md) |
| $k$ | — | ganho do multiplicador | constante em $v_\text{out} = k\thinspace v_x v_y$ | 1/V | [gilbertcell](exemplos/gilbertcell.md) |
| $R_B$ | — | resistência de base | resistor externo ligado à base | Ω | [555int](exemplos/555int.md) |
| $R_b$ | — | resistência de base | resistor externo ligado à base | Ω | [555dutycycle](exemplos/555dutycycle.md) |
| $R_{B1}$ | — | resistência de base 1 | resistor externo ligado à base (do elemento de número 1) | Ω | [multivib-a](exemplos/multivib-a.md) |
| $R_{B2}$ | — | resistência de base 2 | resistor externo ligado à base (do elemento de número 2) | Ω | [multivib-a](exemplos/multivib-a.md) |
| $R_C$ | — | resistência de coletor | resistor externo ligado ao coletor | Ω | [ceamp](exemplos/ceamp.md) |
| $R_E$ | — | resistência de emissor | resistor externo ligado ao emissor | Ω | [ceamp](exemplos/ceamp.md) |
| $r_e$ | — | resistência de emissor | resistência dinâmica do emissor, $V_T/I_E$ | Ω | [ceamp](exemplos/ceamp.md) |
| $R_T$ | — | resistência da cauda | resistor comum aos emissores do par diferencial, que fixa a corrente da cauda | Ω | [trans-diffamp](exemplos/trans-diffamp.md) |
| $R_x$ | — | resistência de degeneração do canal X | resistor entre os emissores do par do canal X, que converte tensão em corrente | Ω | [gilbertcell](exemplos/gilbertcell.md) |
| $R_y$ | — | resistência de degeneração do canal Y | resistor entre os emissores do par do canal Y, que converte tensão em corrente | Ω | [gilbertcell](exemplos/gilbertcell.md) |
| $V_A$ | — | tensão de Early | mede a inclinação das curvas $I_C \times V_{CE}$: $r_o = V_A/I_C$ | V | [currentsrc](exemplos/currentsrc.md) |
| $V_{AF}$ | — | tensão de Early direta | tensão de Early do transistor na operação normal (direta) | V | [early](exemplos/early.md) |
| $V_{AR}$ | — | tensão de Early reversa | tensão de Early do transistor ligado ao contrário | V | [early](exemplos/early.md) |
| $V_B$ | — | tensão do terminal base | tensão da base em relação à terra | V | [555saw](exemplos/555saw.md) |
| $v_B$ | — | tensão instantânea da base | tensão da base em relação à terra em cada instante | V | [3phasewye](exemplos/3phasewye.md) |
| $v_b$ | — | tensão de base de pequeno sinal | variação da tensão da base em torno do ponto de operação | V | [ceamp](exemplos/ceamp.md) |
| $V_{BB}$ | — | tensão entre as bases | tensão aplicada entre as duas bases do transistor unijunção | V | [ujtosc](exemplos/ujtosc.md) |
| $V_{BC}$ | `Vbc` | tensão base-coletor | tensão entre a base e o coletor | V | [ceamp](exemplos/ceamp.md) |
| $V_{BE}$ | `Vbe` | tensão base-emissor | tensão entre a base e o emissor | V | [ceamp](exemplos/ceamp.md) |
| $v_{\text{be}}$ | — | tensão base-emissor de pequeno sinal | variação de $V_{BE}$ em torno do ponto de operação | V | [ceamp](exemplos/ceamp.md) |
| $V_C$ | — | tensão do terminal coletor | tensão do coletor em relação à terra | V | [ceamp](exemplos/ceamp.md) |
| $v_c$ | — | tensão de coletor de pequeno sinal | variação da tensão do coletor em torno do ponto de operação | V | [ceamp](exemplos/ceamp.md) |
| $V_{CB}$ | `Vcb` | tensão coletor-base | tensão entre o coletor e a base | V | [early](exemplos/early.md) |
| $V_{CC}$ | — | tensão de alimentação do coletor | tensão contínua que alimenta o circuito pelo coletor | V | [555dutycycle](exemplos/555dutycycle.md) |
| $V_{CE}$ | `Vce` | tensão coletor-emissor | tensão entre o coletor e o emissor | V | [currentsrc](exemplos/currentsrc.md) |
| $V_{CE(\text{sat})}$ | — | tensão de saturação coletor-emissor | valor pequeno de $V_{CE}$ com o transistor saturado | V | [currentsrc](exemplos/currentsrc.md) |
| $V_{CE(\text{sat})2}$ | — | tensão de saturação do segundo transistor | $V_{CE}$ do transistor de saída do par Darlington quando saturado | V | [darlington](exemplos/darlington.md) |
| $v_{\text{cm}}$ | — | tensão de modo comum | média das duas entradas do par diferencial, $(v_1 + v_2)/2$ | V | [trans-diffamp](exemplos/trans-diffamp.md) |
| $v_d$ | — | tensão diferencial | diferença entre as duas entradas do par diferencial, $v_1 - v_2$ | V | [amp-diff](exemplos/amp-diff.md) |
| $V_E$ | — | tensão do terminal emissor | tensão do emissor em relação à terra | V | [currentsrc](exemplos/currentsrc.md) |
| $v_E$ | — | tensão instantânea do emissor | tensão do emissor em relação à terra em cada instante | V | [follower](exemplos/follower.md) |
| $v_e$ | — | tensão de emissor de pequeno sinal | variação da tensão do emissor em torno do ponto de operação | V | [follower](exemplos/follower.md) |
| $V_{EB}$ | `Veb` | tensão emissor-base | tensão entre o emissor e a base (referência do PNP) | V | [555saw](exemplos/555saw.md) |
| $v_x$ | — | tensão de entrada X | tensão aplicada ao conversor tensão-corrente do canal X | V | [gilbertcell](exemplos/gilbertcell.md) |
| $v_y$ | — | tensão de entrada Y | tensão aplicada ao conversor tensão-corrente do canal Y | V | [gilbertcell](exemplos/gilbertcell.md) |
| $x$ | — | entrada X | tensão aplicada à entrada X do multiplicador (a senoide) | V | [gilbertcell](exemplos/gilbertcell.md) |
| $y$ | — | entrada Y | tensão aplicada à entrada Y do multiplicador (a onda triangular) | V | [gilbertcell](exemplos/gilbertcell.md) |
| $\beta$ | `Beta`, `hFE` | ganho de corrente | razão $I_C/I_B$ do transistor bipolar | — | [ceamp](exemplos/ceamp.md) |
| $\beta_1$ | — | ganho de corrente do transistor 1 | ganho de corrente $\beta$ do transistor de número 1 | — | [darlington](exemplos/darlington.md) |
| $\beta_2$ | — | ganho de corrente do transistor 2 | ganho de corrente $\beta$ do transistor de número 2 | — | [darlington](exemplos/darlington.md) |
| $\beta_{\text{eq}}$ | — | ganho de corrente equivalente | ganho de corrente do conjunto de transistores (no Darlington, $\beta_1\beta_2$) | — | [darlington](exemplos/darlington.md) |
| $\beta_R$ | — | ganho de corrente reverso | $\beta$ do transistor ligado ao contrário (coletor e emissor trocados) | — | [ceamp](exemplos/ceamp.md) |

## Transistores de efeito de campo

| Símbolo | Grafias | Nome | Significado | Unidade | Exemplo |
|---------|---------|------|-------------|---------|---------|
| $A$ | — | amplitude do sinal | valor de pico da senoide de entrada | V | [samplenhold](exemplos/samplenhold.md) |
| $C_{\text{db}}$ | — | capacitância dreno-corpo | capacitância de junção entre o dreno e o corpo | F | [cmosinvertercap](exemplos/cmosinvertercap.md) |
| $C_{\text{gd}}$ | — | capacitância porta-dreno | capacitância parasita entre a porta e o dreno | F | [cmosinvertercap](exemplos/cmosinvertercap.md) |
| $C_{\text{gs}}$ | — | capacitância porta-fonte | capacitância parasita entre a porta e a fonte | F | [cmosinvertercap](exemplos/cmosinvertercap.md) |
| $C_{\text{sb}}$ | — | capacitância fonte-corpo | capacitância de junção entre a fonte e o corpo | F | [cmosinvertercap](exemplos/cmosinvertercap.md) |
| $G_{\text{ds}}$ | — | condutância dreno-fonte | $\partial I_D/\partial V_{DS}$: inverso da resistência do canal | S | [volume](exemplos/volume.md) |
| $g_{\text{ds}}$ | — | condutância de saída | inverso da resistência de saída do dispositivo | S | [nmosfet](exemplos/nmosfet.md) |
| $g_{\text{fuga}}$ | — | condutância de fuga | condutância da chave aberta, que deixa o capacitor descarregar | S | [samplenhold](exemplos/samplenhold.md) |
| $g_m$ | `gm` | transcondutância | quanto a corrente de saída varia por volt na entrada, $\partial I_D/\partial V_{GS}$ | S | [ccvccs](exemplos/ccvccs.md) |
| $I_{\text{alim}}$ | — | corrente de alimentação | corrente que o inversor puxa da fonte $V_{DD}$ | A | [cmosinverter](exemplos/cmosinverter.md) |
| $I_D$ | `Id` | corrente de dreno | corrente que entra pelo dreno do transistor de efeito de campo | A | [jfetamp](exemplos/jfetamp.md) |
| $I_{DSS}$ | — | corrente de dreno de saturação | corrente de dreno com $V_{GS}=0$ | A | [jfetamp](exemplos/jfetamp.md) |
| $I_{\text{fuga}}$ | — | corrente de fuga | corrente que escoa do capacitor de retenção pela chave aberta | A | [samplenhold](exemplos/samplenhold.md) |
| $I_{\text{pico}}$ | — | corrente de pico de curto-circuito | maior corrente da alimentação durante a transição, quando o NMOS e o PMOS conduzem ao mesmo tempo | A | [cmosinverter](exemplos/cmosinverter.md) |
| $L$ | — | comprimento do canal | comprimento do canal do MOSFET; com $W$, define a razão $W/L$ | m | [cmostransgate](exemplos/cmostransgate.md) |
| $R_D$ | — | resistência de dreno | resistor ligado entre a alimentação e o dreno | Ω | [jfetamp](exemplos/jfetamp.md) |
| $R_{\text{ds}}$ | — | resistência dreno-fonte | resistência do canal entre o dreno e a fonte | Ω | [volume](exemplos/volume.md) |
| $R_n$ | — | resistência do canal do NMOS | resistência do NMOS da porta de transmissão quando conduz | Ω | [cmostransgate](exemplos/cmostransgate.md) |
| $r_o$ | `ro` | resistência de saída | resistência incremental vista pelo dreno (ou coletor) | Ω | [currentsrc](exemplos/currentsrc.md) |
| $R_p$ | — | resistência do canal do PMOS | resistência do PMOS da porta de transmissão quando conduz | Ω | [cmostransgate](exemplos/cmostransgate.md) |
| $R_S$ | — | resistência de fonte | resistor ligado entre a fonte e a terra | Ω | [jfetcurrentsrc](exemplos/jfetcurrentsrc.md) |
| $t_d$ | — | atraso | tempo que a tensão do capacitor leva para cruzar o limiar da porta seguinte | s | [delayrc](exemplos/delayrc.md) |
| $t_w$ | — | largura do pulso | tempo em que a tensão fica acima do limiar da porta | s | [leadingedge](exemplos/leadingedge.md) |
| $V_D$ | — | tensão do terminal dreno | tensão do dreno em relação à terra | V | [moscurrentramp](exemplos/moscurrentramp.md) |
| $V_{DD}$ | — | tensão de alimentação | tensão contínua que alimenta o circuito (a ligada aos drenos dos transistores) | V | [cmosinverter](exemplos/cmosinverter.md) |
| $V_{DS}$ | `Vds` | tensão dreno-fonte | tensão entre o dreno e a fonte | V | [cmostransgate](exemplos/cmostransgate.md) |
| $V_{DS,\text{min}}$ | — | tensão dreno-fonte mínima | menor $V_{DS}$ com que a fonte de corrente ainda regula (limite de conformidade) | V | [moscurrentsrc](exemplos/moscurrentsrc.md) |
| $V_{DS2}$ | — | tensão dreno-fonte de M2 | tensão sobre o transistor M2, que copia a corrente | V | [mosmirror](exemplos/mosmirror.md) |
| $V_f$ | — | tensão final | tensão do trilho para o qual o capacitor tende | V | [delayrc](exemplos/delayrc.md) |
| $V_G$ | — | tensão do terminal porta | tensão da porta em relação à terra | V | [pmosfet](exemplos/pmosfet.md) |
| $v_G$ | — | tensão instantânea da porta | tensão da porta em relação à terra em cada instante | V | [cmosinverterslow](exemplos/cmosinverterslow.md) |
| $V_{GS}$ | `Vgs` | tensão porta-fonte | tensão entre a porta e a fonte | V | [3-f221](exemplos/3-f221.md) |
| $V_{GS,n}$ | — | tensão porta-fonte do NMOS | tensão entre a porta e a fonte do transistor NMOS | V | [cmosinverter](exemplos/cmosinverter.md) |
| $V_i$ | — | tensão inicial | tensão do capacitor no início da transição | V | [delayrc](exemplos/delayrc.md) |
| $V_M$ | — | limiar de comutação | tensão de entrada em que a saída do inversor muda de estado (as duas correntes de saturação se igualam) | V | [cmosinverter](exemplos/cmosinverter.md) |
| $V_{\text{ov}}$ | `Vov` | tensão de sobrecarga | $V_{GS} - V_t$: quanto a porta passa do limiar | V | [moscurrentramp](exemplos/moscurrentramp.md) |
| $V_p$ | — | tensão de pico do pulso | valor que a tensão atinge no instante da borda de subida | V | [leadingedge](exemplos/leadingedge.md) |
| $V_S$ | — | tensão do terminal fonte | tensão do terminal fonte (*source*) em relação à terra | V | [jfetamp](exemplos/jfetamp.md) |
| $V_{SD}$ | `Vsd` | tensão fonte-dreno | tensão entre a fonte e o dreno (referência do PMOS) | V | [pmosfet](exemplos/pmosfet.md) |
| $V_{SG}$ | `Vsg` | tensão fonte-porta | tensão entre a fonte e a porta (referência do PMOS) | V | [3-f211](exemplos/3-f211.md) |
| $V_{SG,p}$ | — | tensão fonte-porta do PMOS | tensão entre a fonte e a porta do transistor PMOS | V | [cmosinverter](exemplos/cmosinverter.md) |
| $V_t$ | `Vt` | tensão de limiar | $V_{GS}$ a partir da qual o canal do MOSFET começa a conduzir | V | [3-cgand](exemplos/3-cgand.md) |
| $V_{t,L}$ | — | tensão de limiar da carga | tensão de limiar do transistor de carga | V | [nmosinverter2](exemplos/nmosinverter2.md) |
| $V_{\text{th}}$ | — | limiar da porta seguinte | tensão em que o inversor seguinte muda de estado | V | [delayrc](exemplos/delayrc.md) |
| $V_{\text{th}}$ | `Vth` | tensão de limiar | $V_{GS}$ a partir da qual o canal do MOSFET começa a conduzir | V | [tdrelax](exemplos/tdrelax.md) |
| $V_{\text{tn}}$ | — | tensão de limiar do NMOS | tensão porta-fonte a partir da qual o NMOS conduz | V | [cmosinverter](exemplos/cmosinverter.md) |
| $V_{\text{tp}}$ | — | módulo da tensão de limiar do PMOS | tensão fonte-porta a partir da qual o PMOS conduz (o limiar é $-V_{tp}$) | V | [cmosinverter](exemplos/cmosinverter.md) |
| $W$ | — | largura do canal | largura do canal do MOSFET; com $L$, define a razão $W/L$ | m | [cmostransgate](exemplos/cmostransgate.md) |
| $Z_S$ | — | impedância de fonte | impedância do ramo de fonte: $R_S$ em paralelo com o capacitor de desvio | Ω | [mosfetamp](exemplos/mosfetamp.md) |
| $\beta$ | — | parâmetro de transcondutância | constante da lei quadrática do MOSFET: $I_D = \tfrac12\beta(V_{GS}-V_t)^2$ | A/V² | [cmosinverter](exemplos/cmosinverter.md) |
| $\beta_n$ | — | parâmetro de transcondutância do NMOS | constante $\beta$ da lei quadrática do transistor NMOS | A/V² | [cmosinverter](exemplos/cmosinverter.md) |
| $\beta_p$ | — | parâmetro de transcondutância do PMOS | constante $\beta$ da lei quadrática do transistor PMOS | A/V² | [cmosinverter](exemplos/cmosinverter.md) |
| $\lambda$ | — | modulação do comprimento do canal | parâmetro que dá à corrente de saturação do MOSFET uma pequena inclinação com $V_{DS}$ ($0$: sem inclinação) | 1/V | [cmosinverter](exemplos/cmosinverter.md) |
| $\tau_{\text{ef}}$ | — | constante de tempo efetiva | constante de tempo $(R+R_{on})C$ do circuito, com a resistência do canal | s | [leadingedge](exemplos/leadingedge.md) |
| $\varepsilon$ | — | erro de amostragem | diferença entre a entrada e o valor guardado, $\tau\thinspace dv_{in}/dt$ | V | [samplenhold](exemplos/samplenhold.md) |
| $\varepsilon_{\text{max}}$ | — | erro máximo de amostragem | maior valor do erro de amostragem, na maior inclinação da entrada | V | [samplenhold](exemplos/samplenhold.md) |

## Amplificadores operacionais

| Símbolo | Grafias | Nome | Significado | Unidade | Exemplo |
|---------|---------|------|-------------|---------|---------|
| $A$ | — | ganho | razão entre a saída e a entrada de um amplificador | — | [amp-follower](exemplos/amp-follower.md) |
| $A_v$ | — | ganho de tensão | razão entre a tensão de saída e a de entrada | — | [cc2imp](exemplos/cc2imp.md) |
| $C_c$ | — | capacitor de compensação | capacitor interno que estabiliza o amplificador e limita a sua velocidade | F | [opint-slew](exemplos/opint-slew.md) |
| $I_{\text{lim}}$ | — | corrente limite | valor em que a corrente é limitada pelo circuito | A | [opint-current](exemplos/opint-current.md) |
| $I_{\text{max}}$ | — | corrente máxima | maior corrente que o estágio consegue fornecer | A | [opint-slew](exemplos/opint-slew.md) |
| $K$ | — | fator de escala logarítmico | constante que multiplica o logaritmo na saída ($16\thinspace V_T$) | V | [logconvert](exemplos/logconvert.md) |
| $R_a$ | — | resistência de ganho do segundo estágio | resistor de realimentação do segundo amplificador, que fixa o ganho $1 + R_a/R_b$ | Ω | [logconvert](exemplos/logconvert.md) |
| $R_a$ | — | resistência de referência | resistor que liga a entrada não inversora à tensão de referência | Ω | [amp-schmitt](exemplos/amp-schmitt.md) |
| $R_b$ | — | resistência de realimentação do segundo estágio | resistor entre a entrada inversora e a terra no segundo amplificador | Ω | [logconvert](exemplos/logconvert.md) |
| $R_b$ | — | resistência do divisor de referência | resistor que liga a entrada não inversora à referência negativa | Ω | [amp-schmitt](exemplos/amp-schmitt.md) |
| $R_f$ | — | resistência de realimentação | resistor que liga a saída à entrada inversora | Ω | [allpass1](exemplos/allpass1.md) |
| $R_g$ | — | resistência do girador | resistor ligado em série com a fonte, no circuito que simula um indutor | Ω | [gyrator](exemplos/gyrator.md) |
| $R_k$ | — | resistência da entrada k | resistor em série com a $k$-ésima fonte | Ω | [amp-sum](exemplos/amp-sum.md) |
| $S$ | — | inclinação da rampa | taxa de variação da saída do integrador, $15\ \text{V}/(RC)$ | V/s | [delta-pwm](exemplos/delta-pwm.md) |
| $V_+$ | — | tensão da entrada não inversora | tensão do terminal $+$ do amplificador operacional | V | [allpass1](exemplos/allpass1.md) |
| $v_+$ | — | tensão instantânea da entrada não inversora | tensão do terminal $+$ do amplificador operacional em cada instante | V | [triangle](exemplos/triangle.md) |
| $V_-$ | — | tensão da entrada inversora | tensão do terminal $-$ do amplificador operacional | V | [amp-noninvert](exemplos/amp-noninvert.md) |
| $V_{BE,\text{ref}}$ | — | tensão base-emissor de referência | tensão base-emissor do transistor de referência de corrente | V | [logconvert](exemplos/logconvert.md) |
| $V_c$ | — | tensão de modo comum | senoide de $60\ \text{Hz}$ presente nas duas entradas, que o amplificador diferencial cancela | V | [amp-diff](exemplos/amp-diff.md) |
| $V_{\text{dec}}$ | — | tensão decodificada | tensão obtida pela integração da saída, que acompanha a entrada | V | [delta-pwm](exemplos/delta-pwm.md) |
| $V_H$ | — | histerese | diferença entre os limiares superior e inferior do gatilho Schmitt | V | [schmitt](exemplos/schmitt.md) |
| $V_k$ | — | tensão da fonte k | tensão da $k$-ésima fonte somada pelo circuito | V | [amp-sum](exemplos/amp-sum.md) |
| $V_L$ | — | tensão na carga | tensão sobre a carga da fonte de corrente Howland | V | [howland](exemplos/howland.md) |
| $V_{LI}$ | — | limiar inferior | tensão de entrada em que o gatilho muda a saída descendo | V | [schmitt](exemplos/schmitt.md) |
| $V_{\text{lim}}$ | — | tensão limite | valor em que a tensão de um nó é limitada pelo circuito | V | [trianglevco](exemplos/trianglevco.md) |
| $V_{LS}$ | — | limiar superior | tensão de entrada em que o gatilho muda a saída subindo | V | [schmitt](exemplos/schmitt.md) |
| $V_M$ | — | tensão no ponto M | tensão no nó entre o resistor e o capacitor, copiada pelo seguidor | V | [gyrator](exemplos/gyrator.md) |
| $V_{\text{max}}$ | — | limite superior da saída | maior tensão que a saída pode atingir | V | [opamp](exemplos/opamp.md) |
| $V_{\text{min}}$ | — | limite inferior da saída | menor tensão que a saída pode atingir | V | [opamp](exemplos/opamp.md) |
| $V_{\text{ofs}}$ | — | tensão de deslocamento | tensão que desloca o ponto de operação | V | [deltasigma](exemplos/deltasigma.md) |
| $V_{\text{sat}}$ | — | tensão de saturação | valor em que a saída do amplificador fica travada ($\pm V_\text{sat}$) | V | [amp-schmitt](exemplos/amp-schmitt.md) |
| $V_x$ | — | tensão do primeiro estágio | tensão na saída do primeiro estágio logarítmico | V | [logconvert](exemplos/logconvert.md) |
| $Z_f$ | — | impedância de realimentação | impedância que liga a saída à entrada inversora | Ω | [invertamp](exemplos/invertamp.md) |
| $\beta$ | — | fração de realimentação | parte da saída que volta à entrada não inversora ou inversora | — | [relaxosc](exemplos/relaxosc.md) |

## Filtros e resposta em frequência

| Símbolo | Grafias | Nome | Significado | Unidade | Exemplo |
|---------|---------|------|-------------|---------|---------|
| $B$ | — | largura de banda | faixa de frequências em que o filtro responde, $f_0/Q$ | Hz | [actbutterband](exemplos/actbutterband.md) |
| $B_{\text{eq}}$ | — | largura de banda equivalente | largura da banda de ruído que passa o filtro | Hz | [actbutterband](exemplos/actbutterband.md) |
| $C_f$ | — | capacitor de realimentação | capacitor entre a saída e a entrada inversora | F | [actbutterlo](exemplos/actbutterlo.md) |
| $C_g$ | — | capacitor à terra | capacitor ligado entre a entrada e a terra | F | [actbutterlo](exemplos/actbutterlo.md) |
| $f_0$ | — | frequência de ressonância | frequência em que o circuito responde ao máximo | Hz | [actbutterband](exemplos/actbutterband.md) |
| $f_c$ | — | frequência de corte | frequência em que o ganho cai a $1/\sqrt{2}$ do valor da banda passante (−3 dB) | Hz | [actbutterhi](exemplos/actbutterhi.md) |
| $f_d$ | — | frequência amortecida | $\omega_d/(2\pi)$: frequência da oscilação subamortecida | Hz | [lrc](exemplos/lrc.md) |
| $f_N$ | — | frequência de Nyquist | metade da frequência de amostragem | Hz | [actbutterband](exemplos/actbutterband.md) |
| $g_k$ | — | coeficiente do componente k | valor normalizado do componente de índice $k$ do filtro | — | [butter10hi](exemplos/butter10hi.md) |
| $H$ | — | função de transferência | razão entre a saída e a entrada de um filtro, em função da frequência | — | [actbutterlo](exemplos/actbutterlo.md) |
| $H_{\text{pa}}$ | — | resposta passa-altas | função de transferência do filtro passa-altas | — | [actbutterhi](exemplos/actbutterhi.md) |
| $H_{\text{pb}}$ | — | resposta passa-baixas | função de transferência do filtro passa-baixas | — | [actbutterhi](exemplos/actbutterhi.md) |
| $K$ | — | ganho do amplificador | ganho não inversor $1 + R_a/R_b$ do amplificador do filtro | — | [filt-vcvs-hipass](exemplos/filt-vcvs-hipass.md) |
| $Q$ | — | fator de qualidade | mede o quão seletivo é um circuito ressonante: $f_0/B$, razão entre a frequência central e a largura de banda | — | [actbutterband](exemplos/actbutterband.md) |
| $Q_k$ | — | fator de qualidade da seção k | fator de qualidade da seção de índice $k$ do filtro | — | [actbutterlo](exemplos/actbutterlo.md) |
| $R_c$ | — | resistência crítica | valor de $R$ que separa o regime oscilatório do não oscilatório | Ω | [lrc-critical](exemplos/lrc-critical.md) |
| $R_{\text{res}}$ | — | resistência em série | resistor externo em série com o capacitor do filtro | Ω | [switchfilter](exemplos/switchfilter.md) |
| $s$ | — | variável de Laplace | frequência complexa; no regime senoidal, $s = j\omega$ | 1/s | [3motor](exemplos/3motor.md) |
| $S_{\text{in}}$ | — | densidade espectral de entrada | potência do sinal de entrada por unidade de frequência | V²/Hz | [butter10hiaud](exemplos/butter10hiaud.md) |
| $S_{\text{out}}$ | — | densidade espectral de saída | potência do sinal de saída por unidade de frequência | V²/Hz | [butter10hiaud](exemplos/butter10hiaud.md) |
| $\alpha$ | — | fator de amortecimento | $R/(2L)$: taxa com que a oscilação decai, como $e^{-\alpha t}$ | 1/s | [coupled1](exemplos/coupled1.md) |
| $\omega_0$ | — | frequência angular natural | frequência angular em que o circuito ressoa, $1/\sqrt{LC}$ | rad/s | [bandnoise](exemplos/bandnoise.md) |
| $\omega_1$ | — | frequência angular inferior | frequência angular de corte inferior da faixa | rad/s | [butterbandstop](exemplos/butterbandstop.md) |
| $\omega_2$ | — | frequência angular superior | frequência angular de corte superior da faixa | rad/s | [butterbandstop](exemplos/butterbandstop.md) |
| $\omega_d$ | — | frequência angular amortecida | $\sqrt{\omega_0^2-\alpha^2}$: frequência da oscilação subamortecida | rad/s | [lrc](exemplos/lrc.md) |
| $\tau_g$ | — | atraso de grupo | $-d\varphi/d\omega$: atraso que o filtro impõe ao envelope do sinal | s | [allpass1](exemplos/allpass1.md) |
| $\theta_k$ | — | ângulo k | ângulo $(2k-1)\pi/(2n)$ do polo de índice $k$ | rad | [actbutterlo](exemplos/actbutterlo.md) |
| $\zeta$ | — | fator de amortecimento relativo | $\alpha/\omega_0$: com $\zeta<1$ o circuito oscila, com $\zeta=1$ é crítico e com $\zeta>1$ não oscila | — | [inductkick](exemplos/inductkick.md) |

## Lógica e digital

| Símbolo | Grafias | Nome | Significado | Unidade | Exemplo |
|---------|---------|------|-------------|---------|---------|
| $A$ | — | entrada lógica A | primeira entrada da porta ou do circuito lógico | — | [3-cgand](exemplos/3-cgand.md) |
| $a$ | — | bit a | bit da palavra que escolhe o nível da componente em fase | — | [qam-256](exemplos/qam-256.md) |
| $a$ | — | segmento a | primeiro dos sete segmentos do mostrador | — | [7segdecoder](exemplos/7segdecoder.md) |
| $A_0$ | — | bit 0 da entrada A | bit de índice 0 do número de 4 bits A | — | [adder4-sc](exemplos/adder4-sc.md) |
| $A_0$ | — | sinal lógico A0 | sinal lógico $A_0$ (bit ou elemento de índice 0) do circuito | — | [alu74181](exemplos/alu74181.md) |
| $a_0$ | — | bit a0 | bit menos significativo do número $a$ | — | [plot2d-checker](exemplos/plot2d-checker.md) |
| $A_1$ | — | sinal lógico A1 | sinal lógico $A_1$ (bit ou elemento de índice 1) do circuito | — | [decoder](exemplos/decoder.md) |
| $A_3$ | — | bit 3 da entrada A | bit de índice 3 do número de 4 bits A | — | [adder4-sc](exemplos/adder4-sc.md) |
| $A_3$ | — | sinal lógico A3 | sinal lógico $A_3$ (bit ou elemento de índice 3) do circuito | — | [alu74181](exemplos/alu74181.md) |
| $A_7$ | — | sinal lógico A7 | sinal lógico $A_7$ (bit ou elemento de índice 7) do circuito | — | [brentkung](exemplos/brentkung.md) |
| $A_i$ | — | sinal lógico Ai | sinal lógico $A_i$ (bit ou elemento de índice i) do circuito | — | [alu74181](exemplos/alu74181.md) |
| $B$ | — | canal azul | bit de cor azul da palavra lida da ROM | — | [plot2d-smile](exemplos/plot2d-smile.md) |
| $B$ | — | entrada lógica B | segunda entrada da porta ou do circuito lógico | — | [3-cgand](exemplos/3-cgand.md) |
| $B_0$ | — | bit 0 da entrada B | bit de índice 0 do número de 4 bits B | — | [adder4-sc](exemplos/adder4-sc.md) |
| $B_0$ | — | sinal lógico B0 | sinal lógico $B_0$ (bit ou elemento de índice 0) do circuito | — | [alu74181](exemplos/alu74181.md) |
| $b_0$ | — | bit 0 | bit menos significativo do número | — | [dac](exemplos/dac.md) |
| $B_1$ | — | sinal lógico B1 | sinal lógico $B_1$ (bit ou elemento de índice 1) do circuito | — | [digcompare](exemplos/digcompare.md) |
| $b_1$ | — | bit 1 | bit de peso $2^1$ do número | — | [dac](exemplos/dac.md) |
| $b_2$ | — | bit 2 | bit de peso $2^2$ do número | — | [dac](exemplos/dac.md) |
| $B_3$ | — | bit 3 da entrada B | bit de índice 3 do número de 4 bits B | — | [adder4-sc](exemplos/adder4-sc.md) |
| $B_3$ | — | sinal lógico B3 | sinal lógico $B_3$ (bit ou elemento de índice 3) do circuito | — | [alu74181](exemplos/alu74181.md) |
| $b_3$ | — | bit 3 | bit de peso $2^3$ do número | — | [dac](exemplos/dac.md) |
| $B_7$ | — | sinal lógico B7 | sinal lógico $B_7$ (bit ou elemento de índice 7) do circuito | — | [brentkung](exemplos/brentkung.md) |
| $B_i$ | — | sinal lógico Bi | sinal lógico $B_i$ (bit ou elemento de índice i) do circuito | — | [alu74181](exemplos/alu74181.md) |
| $B_{i+1}$ | — | bit i+1 do binário | bit de índice $i+1$ do número em binário | — | [graycode](exemplos/graycode.md) |
| $b_k$ | — | bit k | bit de índice $k$ do número | — | [r2rladder](exemplos/r2rladder.md) |
| $C$ | — | entrada lógica C | terceira entrada ou sinal de relógio do circuito lógico | — | [4way](exemplos/4way.md) |
| $C$ | — | valor C | parte de $N = 16C + F$ formada pelos bits mais significativos | — | [hfadc](exemplos/hfadc.md) |
| $c_i$ | — | vai-um do bit i | bit de transporte que entra no bit de índice $i$ | — | [brentkung](exemplos/brentkung.md) |
| $C_{\text{in}}$ | — | vai-um de entrada | bit de transporte que entra no somador | — | [adder4-sc](exemplos/adder4-sc.md) |
| $C_n$ | — | vai-um de entrada (ativo em nível baixo) | bit de transporte que entra na unidade lógica e aritmética | — | [alu74181](exemplos/alu74181.md) |
| $C_{n+4}$ | — | vai-um de saída (ativo em nível baixo) | bit de transporte que sai da unidade lógica e aritmética | — | [alu74181](exemplos/alu74181.md) |
| $C_{\text{out}}$ | — | vai-um de saída | bit de transporte que sai do somador | — | [adder4-sc](exemplos/adder4-sc.md) |
| $D$ | — | entrada de dados D | entrada de dados do flip-flop ou do circuito lógico | — | [cmosmsff](exemplos/cmosmsff.md) |
| $D_0$ | — | sinal lógico D0 | sinal lógico $D_0$ (bit ou elemento de índice 0) do circuito | — | [7segdecoder](exemplos/7segdecoder.md) |
| $D_1$ | — | entrada de dados 1 | primeira entrada de dados do multiplexador | — | [7segdecoder](exemplos/7segdecoder.md) |
| $D_2$ | — | entrada de dados 2 | segunda entrada de dados do multiplexador | — | [7segdecoder](exemplos/7segdecoder.md) |
| $D_3$ | — | sinal lógico D3 | sinal lógico $D_3$ (bit ou elemento de índice 3) do circuito | — | [7segdecoder](exemplos/7segdecoder.md) |
| $D_{\text{in}}$ | — | entrada serial | bit que entra pelo primeiro flip-flop do registrador de deslocamento | — | [sipo-sr](exemplos/sipo-sr.md) |
| $E$ | — | entrada de habilitação | entrada que liga ou desliga o circuito | — | [mux3state](exemplos/mux3state.md) |
| $e_0$ | — | igualdade do bit 0 | vale $1$ quando os bits $A_0$ e $B_0$ são iguais | — | [digcompare](exemplos/digcompare.md) |
| $e_1$ | — | igualdade do bit 1 | vale $1$ quando os bits $A_1$ e $B_1$ são iguais | — | [digcompare](exemplos/digcompare.md) |
| $e_i$ | — | igualdade do bit i | vale $1$ quando os bits $A_i$ e $B_i$ são iguais | — | [digcompare](exemplos/digcompare.md) |
| $F$ | — | parte fracionária | bits menos significativos do resultado da conversão | — | [hfadc](exemplos/hfadc.md) |
| $F$ | — | resultado | saída de $4$ bits da unidade lógica e aritmética | — | [alu74181](exemplos/alu74181.md) |
| $F_0$ | — | sinal lógico F0 | sinal lógico $F_0$ (bit ou elemento de índice 0) do circuito | — | [alu74181](exemplos/alu74181.md) |
| $f_1$ | — | sinal lógico f1 | sinal lógico $f_1$ (bit ou elemento de índice 1) do circuito | — | [phasecomp](exemplos/phasecomp.md) |
| $f_2$ | — | sinal lógico f2 | sinal lógico $f_2$ (bit ou elemento de índice 2) do circuito | — | [phasecomp](exemplos/phasecomp.md) |
| $F_{210}$ | — | função ternária F210 | função ternária de uma entrada que dá $2$, $1$, $0$ para as entradas $0$, $1$, $2$ | — | [3-invert](exemplos/3-invert.md) |
| $F_{211}$ | — | função ternária F211 | função ternária de uma entrada que dá $2$ para a entrada $0$ e $1$ para as demais | — | [3-f211](exemplos/3-f211.md) |
| $F_{220}$ | — | função ternária F220 | função ternária que dá $2$ para as entradas $0$ e $1$ e $0$ para $2$ | — | [3-f220](exemplos/3-f220.md) |
| $F_{221}$ | — | função ternária F221 | função ternária que dá $2$ para as entradas $0$ e $1$ e $1$ para $2$ | — | [3-f221](exemplos/3-f221.md) |
| $F_3$ | — | sinal lógico F3 | sinal lógico $F_3$ (bit ou elemento de índice 3) do circuito | — | [alu74181](exemplos/alu74181.md) |
| $F_{\text{xyz}}$ | — | função ternária Fxyz | notação das funções ternárias de uma entrada: $x$, $y$, $z$ são as saídas para as entradas $0$, $1$, $2$ | — | [3-f211](exemplos/3-f211.md) |
| $G$ | — | canal verde | bit de cor verde da palavra lida da ROM | — | [plot2d-smile](exemplos/plot2d-smile.md) |
| $G$ | — | código Gray | número em código Gray, em que valores vizinhos diferem em um bit | — | [graycode](exemplos/graycode.md) |
| $G$ | — | geração | indica se um grupo de bits gera um vai-um sozinho | — | [brentkung](exemplos/brentkung.md) |
| $g$ | — | segmento g | um dos sete segmentos do mostrador | — | [7segdecoder](exemplos/7segdecoder.md) |
| $g_{-1}$ | — | geração do bit −1 | vai-um de entrada tratado como um bit extra da árvore | — | [brentkung](exemplos/brentkung.md) |
| $G_0$ | — | sinal lógico G0 | sinal lógico $G_0$ (bit ou elemento de índice 0) do circuito | — | [graycode](exemplos/graycode.md) |
| $g_0$ | — | sinal lógico g0 | sinal lógico $g_0$ (bit ou elemento de índice 0) do circuito | — | [brentkung](exemplos/brentkung.md) |
| $G_1$ | — | sinal lógico G1 | sinal lógico $G_1$ (bit ou elemento de índice 1) do circuito | — | [graycode](exemplos/graycode.md) |
| $g_1$ | — | sinal lógico g1 | sinal lógico $g_1$ (bit ou elemento de índice 1) do circuito | — | [brentkung](exemplos/brentkung.md) |
| $G_2$ | — | sinal lógico G2 | sinal lógico $G_2$ (bit ou elemento de índice 2) do circuito | — | [graycode](exemplos/graycode.md) |
| $G_3$ | — | sinal lógico G3 | sinal lógico $G_3$ (bit ou elemento de índice 3) do circuito | — | [graycode](exemplos/graycode.md) |
| $G_{7:0}$ | — | vai-um final | vai-um que sai do bit $7$, o nono bit da soma | — | [brentkung](exemplos/brentkung.md) |
| $G_i$ | — | sinal lógico Gi | sinal lógico $G_i$ (bit ou elemento de índice i) do circuito | — | [alu74181](exemplos/alu74181.md) |
| $g_i$ | — | geração do bit i | $A_i B_i$: o bit $i$ gera um vai-um sozinho | — | [brentkung](exemplos/brentkung.md) |
| $G_{i-1:0}$ | — | vai-um para o bit i | vai-um que chega ao bit $i$ vindo dos bits $i-1$ a $0$ | — | [brentkung](exemplos/brentkung.md) |
| $G_{i:0}$ | — | vai-um do bit i | vai-um que sai do bit $i$, considerando os bits $i$ a $0$ | — | [brentkung](exemplos/brentkung.md) |
| $G_{i:k}$ | — | geração do grupo i:k | indica se os bits de $k$ a $i$ geram um vai-um sozinhos | — | [brentkung](exemplos/brentkung.md) |
| $G_{k-1:j}$ | — | geração do grupo k−1:j | indica se os bits de $j$ a $k-1$ geram um vai-um sozinhos | — | [brentkung](exemplos/brentkung.md) |
| $I_{IH}$ | — | corrente de entrada em nível alto | corrente que a entrada da porta puxa no nível alto | A | [fanout](exemplos/fanout.md) |
| $I_{IL}$ | — | corrente de entrada em nível baixo | corrente que a entrada da porta fornece no nível baixo | A | [fanout](exemplos/fanout.md) |
| $I_{OH}$ | — | corrente de saída em nível alto | corrente que a saída da porta fornece no nível alto | A | [fanout](exemplos/fanout.md) |
| $I_{OL}$ | — | corrente de saída em nível baixo | corrente que a saída da porta absorve no nível baixo | A | [fanout](exemplos/fanout.md) |
| $J$ | — | entrada J | entrada de ajuste do flip-flop JK | — | [counter](exemplos/counter.md) |
| $K$ | — | entrada K | entrada de limpeza do flip-flop JK | — | [clockedsrff](exemplos/clockedsrff.md) |
| $M$ | — | modo | entrada que escolhe entre operações lógicas e aritméticas | — | [alu74181](exemplos/alu74181.md) |
| $M$ | — | saída maioria | saída da função maioria: vale $1$ se ao menos duas entradas valem $1$ | — | [majority](exemplos/majority.md) |
| $m_k$ | — | mintermo k | termo produto do valor $k$ da entrada | — | [7segdecoder](exemplos/7segdecoder.md) |
| $N$ | — | número de LEDs acesos | quantidade de LEDs da matriz que estão acesos | — | [ledarray](exemplos/ledarray.md) |
| $N$ | — | número de portas ligadas | quantidade de entradas que a saída da porta aciona | — | [fanout](exemplos/fanout.md) |
| $N$ | — | saída da NAND intermediária | nó $N$ interno, saída da NAND das duas entradas | — | [xor](exemplos/xor.md) |
| $N$ | — | valor digital N | número inteiro representado pelos bits de entrada do conversor | — | [dac](exemplos/dac.md) |
| $O$ | — | saída O | saída de dados do registrador | — | [phasecomp](exemplos/phasecomp.md) |
| $P$ | — | propagação | indica se um grupo de bits propaga o vai-um recebido | — | [brentkung](exemplos/brentkung.md) |
| $p_0$ | — | sinal lógico p0 | sinal lógico $p_0$ (bit ou elemento de índice 0) do circuito | — | [brentkung](exemplos/brentkung.md) |
| $p_1$ | — | sinal lógico p1 | sinal lógico $p_1$ (bit ou elemento de índice 1) do circuito | — | [brentkung](exemplos/brentkung.md) |
| $P_i$ | — | sinal lógico Pi | sinal lógico $P_i$ (bit ou elemento de índice i) do circuito | — | [alu74181](exemplos/alu74181.md) |
| $p_i$ | — | propagação do bit i | $A_i \oplus B_i$: o bit $i$ propaga o vai-um recebido | — | [brentkung](exemplos/brentkung.md) |
| $P_{i:k}$ | — | propagação do grupo i:k | indica se os bits de $k$ a $i$ propagam um vai-um recebido | — | [brentkung](exemplos/brentkung.md) |
| $P_k$ | — | entrada paralela k | bit de índice $k$ que é carregado em paralelo | — | [piso-sr](exemplos/piso-sr.md) |
| $P_{k-1:j}$ | — | propagação do grupo k−1:j | indica se os bits de $j$ a $k-1$ propagam um vai-um recebido | — | [brentkung](exemplos/brentkung.md) |
| $Q$ | — | saída Q | saída de um flip-flop ou latch, que guarda o bit armazenado | — | [clockedsrff](exemplos/clockedsrff.md) |
| $Q_0$ | — | sinal lógico Q0 | sinal lógico $Q_0$ (bit ou elemento de índice 0) do circuito | — | [counter](exemplos/counter.md) |
| $Q_1$ | — | sinal lógico Q1 | sinal lógico $Q_1$ (bit ou elemento de índice 1) do circuito | — | [counter](exemplos/counter.md) |
| $Q_2$ | — | sinal lógico Q2 | sinal lógico $Q_2$ (bit ou elemento de índice 2) do circuito | — | [ledflasher](exemplos/ledflasher.md) |
| $Q_3$ | — | sinal lógico Q3 | sinal lógico $Q_3$ (bit ou elemento de índice 3) do circuito | — | [deccounter](exemplos/deccounter.md) |
| $Q_4$ | — | sinal lógico Q4 | sinal lógico $Q_4$ (bit ou elemento de índice 4) do circuito | — | [johnsonctr](exemplos/johnsonctr.md) |
| $Q_5$ | — | sinal lógico Q5 | sinal lógico $Q_5$ (bit ou elemento de índice 5) do circuito | — | [ledflasher](exemplos/ledflasher.md) |
| $Q_6$ | — | sinal lógico Q6 | sinal lógico $Q_6$ (bit ou elemento de índice 6) do circuito | — | [ledflasher](exemplos/ledflasher.md) |
| $Q_7$ | — | sinal lógico Q7 | sinal lógico $Q_7$ (bit ou elemento de índice 7) do circuito | — | [ledflasher](exemplos/ledflasher.md) |
| $Q_8$ | — | sinal lógico Q8 | sinal lógico $Q_8$ (bit ou elemento de índice 8) do circuito | — | [ledflasher](exemplos/ledflasher.md) |
| $Q_9$ | — | sinal lógico Q9 | sinal lógico $Q_9$ (bit ou elemento de índice 9) do circuito | — | [ledflasher](exemplos/ledflasher.md) |
| $Q_j$ | — | saída do bit j | saída $Q$ do flip-flop de índice $j$ | — | [updownctr](exemplos/updownctr.md) |
| $Q_{k-1}$ | — | saída do flip-flop k−1 | saída $Q$ do flip-flop anterior, que é carregada no flip-flop $k$ | — | [piso-sr](exemplos/piso-sr.md) |
| $Q_n$ | — | estado atual | valor da saída $Q$ antes do pulso do relógio | — | [relaytff](exemplos/relaytff.md) |
| $Q_{n+1}$ | — | estado seguinte | valor da saída $Q$ depois do pulso do relógio | — | [relaytff](exemplos/relaytff.md) |
| $R$ | — | canal vermelho | bit de cor vermelho da palavra lida da ROM | — | [plot2d-smile](exemplos/plot2d-smile.md) |
| $R$ | — | entrada R | entrada de limpeza (*reset*) do circuito lógico | — | [clockedsrff](exemplos/clockedsrff.md) |
| $S$ | — | seleção do multiplexador | sinal de seleção: com $S=1$ a saída segue $v_1$ e com $S=0$ segue $v_2$ | — | [mux](exemplos/mux.md) |
| $S$ | — | sinal lógico S | entrada de seleção, de ajuste (*set*) ou de soma, conforme o circuito | — | [adder4-sc](exemplos/adder4-sc.md) |
| $S_0$ | — | sinal lógico S0 | sinal lógico $S_0$ (bit ou elemento de índice 0) do circuito | — | [alu74181](exemplos/alu74181.md) |
| $S_1$ | — | sinal lógico S1 | sinal lógico $S_1$ (bit ou elemento de índice 1) do circuito | — | [alu74181](exemplos/alu74181.md) |
| $S_2$ | — | sinal lógico S2 | sinal lógico $S_2$ (bit ou elemento de índice 2) do circuito | — | [alu74181](exemplos/alu74181.md) |
| $S_3$ | — | sinal lógico S3 | sinal lógico $S_3$ (bit ou elemento de índice 3) do circuito | — | [alu74181](exemplos/alu74181.md) |
| $S_i$ | — | bit i da soma | bit de índice $i$ do resultado da soma | — | [adder4-sc](exemplos/adder4-sc.md) |
| $T_0$ | — | sinal lógico T0 | sinal lógico $T_0$ (bit ou elemento de índice 0) do circuito | — | [synccounter](exemplos/synccounter.md) |
| $T_1$ | — | sinal lógico T1 | sinal lógico $T_1$ (bit ou elemento de índice 1) do circuito | — | [synccounter](exemplos/synccounter.md) |
| $T_2$ | — | sinal lógico T2 | sinal lógico $T_2$ (bit ou elemento de índice 2) do circuito | — | [synccounter](exemplos/synccounter.md) |
| $T_3$ | — | sinal lógico T3 | sinal lógico $T_3$ (bit ou elemento de índice 3) do circuito | — | [synccounter](exemplos/synccounter.md) |
| $T_k$ | — | entrada T do bit k | entrada de inversão do flip-flop de índice $k$ | — | [updownctr](exemplos/updownctr.md) |
| $U$ | — | sentido da contagem | entrada que escolhe contar para cima ($U=1$) ou para baixo ($U=0$) | — | [updownctr](exemplos/updownctr.md) |
| $V_{DAC}$ | — | tensão do conversor D/A | tensão que o conversor digital-analógico entrega | V | [hfadc](exemplos/hfadc.md) |
| $V_{OH}$ | — | tensão de saída em nível alto | tensão da saída no nível lógico alto | V | [fanout](exemplos/fanout.md) |
| $V_{OL}$ | — | tensão de saída em nível baixo | tensão da saída no nível lógico baixo | V | [nmosinverter](exemplos/nmosinverter.md) |
| $X$ | — | entrada X | uma das três entradas X da função maioria | — | [majority](exemplos/majority.md) |
| $X$ | — | propagação de grupo | saída que indica se o grupo de 4 bits propaga um vai-um | — | [alu74181](exemplos/alu74181.md) |
| $x$ | — | entrada ternária x | valor lógico de entrada ($0$, $1$ ou $2$) de uma função ternária | — | [3-cgand](exemplos/3-cgand.md) |
| $Y$ | — | entrada Y | uma das três entradas Y da função maioria | — | [majority](exemplos/majority.md) |
| $Y$ | — | saída lógica Y | saída do circuito lógico | — | [3-cgand](exemplos/3-cgand.md) |
| $Y_0$ | — | sinal lógico Y0 | sinal lógico $Y_0$ (bit ou elemento de índice 0) do circuito | — | [decoder](exemplos/decoder.md) |
| $Y_1$ | — | sinal lógico Y1 | sinal lógico $Y_1$ (bit ou elemento de índice 1) do circuito | — | [decoder](exemplos/decoder.md) |
| $Y_2$ | — | sinal lógico Y2 | sinal lógico $Y_2$ (bit ou elemento de índice 2) do circuito | — | [decoder](exemplos/decoder.md) |
| $Y_3$ | — | sinal lógico Y3 | sinal lógico $Y_3$ (bit ou elemento de índice 3) do circuito | — | [decoder](exemplos/decoder.md) |
| $Y_k$ | — | saída k | saída de índice $k$ do decodificador | — | [decoder](exemplos/decoder.md) |
| $Y_{NOR}$ | — | saída NOR | saída da função NOR de duas entradas | — | [eclnor](exemplos/eclnor.md) |
| $Y_{OR}$ | — | saída OR | saída da função OR de duas entradas | — | [eclnor](exemplos/eclnor.md) |
| $Z$ | — | entrada Z | uma das três entradas Z da função maioria | — | [majority](exemplos/majority.md) |
| $Z_s$ | — | conjunto de apagados | conjunto de valores para os quais o segmento fica apagado | — | [7segdecoder](exemplos/7segdecoder.md) |

## Temporizadores e osciladores

| Símbolo | Grafias | Nome | Significado | Unidade | Exemplo |
|---------|---------|------|-------------|---------|---------|
| $A$ | — | amplitude da oscilação | valor de pico da tensão oscilante, que cresce até o limite do inversor | V | [crystalosc](exemplos/crystalosc.md) |
| $a$ | — | razão de resistências | razão $R_{17}/R$ do laço do oscilador | — | [sine](exemplos/sine.md) |
| $A_0$ | — | amplitude inicial | amplitude da oscilação quando ela nasce do ruído | V | [crystalosc](exemplos/crystalosc.md) |
| $f_p$ | — | frequência de ressonância paralela | frequência em que o cristal ressoa em paralelo, um pouco acima de $f_s$ | Hz | [crystalosc](exemplos/crystalosc.md) |
| $f_s$ | — | frequência de ressonância série | frequência em que o ramo em série $L C_s$ ressoa | Hz | [crystalosc](exemplos/crystalosc.md) |
| $N$ | — | fator de multiplicação | razão entre a frequência de saída e a de entrada | — | [freqdouble](exemplos/freqdouble.md) |
| $Q_L$ | — | fator de qualidade com carga | fator de qualidade do circuito ressonante com a carga $R_L$ em paralelo | — | [eclosc](exemplos/eclosc.md) |
| $Q_u$ | — | fator de qualidade sem carga | fator de qualidade do circuito ressonante só com a perda $R_p$, $R_p/(\omega_0 L)$ | — | [eclosc](exemplos/eclosc.md) |
| $R_A$ | — | resistência R_A | resistor do temporizador que carrega o capacitor | Ω | [555int](exemplos/555int.md) |
| $R_{\text{carga}}$ | — | resistência de carga do capacitor | resistor pelo qual o capacitor se carrega | Ω | [555lowduty](exemplos/555lowduty.md) |
| $R_{\text{descarga}}$ | — | resistência de descarga do capacitor | resistor pelo qual o capacitor se descarrega | Ω | [555lowduty](exemplos/555lowduty.md) |
| $R_{\text{lento}}$ | — | resistência da subida lenta | resistor que carrega o capacitor durante a rampa lenta | Ω | [sawtooth](exemplos/sawtooth.md) |
| $R_p$ | — | resistência de perda | resistor em paralelo que representa as perdas do circuito ressonante | Ω | [eclosc](exemplos/eclosc.md) |
| $R_{\text{pot}}$ | — | resistência do potenciômetro | resistência total do potenciômetro | Ω | [555dutycycle](exemplos/555dutycycle.md) |
| $R_{\text{rápido}}$ | — | resistência da descida rápida | resistor que descarrega o capacitor durante a rampa rápida | Ω | [sawtooth](exemplos/sawtooth.md) |
| $t_{\text{alto}}$ | — | tempo em nível alto | duração do semiciclo em que a saída fica em nível alto | s | [traffic](exemplos/traffic.md) |
| $t_{\text{baixo}}$ | — | tempo em nível baixo | duração do semiciclo em que a saída fica em nível baixo | s | [traffic](exemplos/traffic.md) |
| $t_{\text{carga}}$ | — | tempo de carga | duração da carga do capacitor | s | [555saw](exemplos/555saw.md) |
| $t_{\text{desc}}$ | — | tempo de descarga | duração da descarga do capacitor | s | [555saw](exemplos/555saw.md) |
| $t_h$ | — | tempo em nível alto | duração do semiciclo em que a saída fica em nível alto | s | [555dutycycle](exemplos/555dutycycle.md) |
| $t_l$ | — | tempo em nível baixo | duração do semiciclo em que a saída fica em nível baixo | s | [555dutycycle](exemplos/555dutycycle.md) |
| $t_{\text{lento}}$ | — | tempo da rampa lenta | duração da rampa lenta do dente de serra | s | [sawtooth](exemplos/sawtooth.md) |
| $t_{\text{rápido}}$ | — | tempo da rampa rápida | duração da rampa rápida do dente de serra | s | [sawtooth](exemplos/sawtooth.md) |
| $U_{\text{out}}$ | — | amplitude da saída | amplitude da tensão de saída do oscilador | V | [eclosc](exemplos/eclosc.md) |
| $U_{\text{sat}}$ | — | tensão de saturação | limite da tensão de saída imposto pela saturação do transistor Q2 | V | [eclosc](exemplos/eclosc.md) |
| $V_{B0}$ | — | tensão inicial da base | tensão da base no instante em que o transistor passa ao corte (módulo próximo de $V_{CC}$) | V | [multivib-mono](exemplos/multivib-mono.md) |
| $V_c$ | — | tensão de controle | tensão que ajusta a frequência de um oscilador controlado por tensão | V | [555pulsemod](exemplos/555pulsemod.md) |
| $v_o$ | — | tensão de saída do oscilador | tensão na saída $v_o$ do circuito, onde o capacitor se liga | V | [inv-osc](exemplos/inv-osc.md) |
| $v_q$ | — | tensão instantânea da onda quadrada | saída do comparador em cada instante | V | [triangle](exemplos/triangle.md) |
| $v_t$ | — | tensão instantânea da triangular | onda triangular de alta frequência em cada instante | V | [classd](exemplos/classd.md) |
| $x$ | — | frequência normalizada | produto $s\tau$ da variável de Laplace pela constante de tempo | — | [sine](exemplos/sine.md) |
| $\omega_s$ | — | frequência angular de ressonância série | $2\pi f_s$ | rad/s | [crystalosc2](exemplos/crystalosc2.md) |
| $\tau_g$ | — | constante de tempo de crescimento | tempo em que a amplitude da oscilação cresce por um fator $e$ | s | [crystalosc](exemplos/crystalosc.md) |

## Conversores de potência

| Símbolo | Grafias | Nome | Significado | Unidade | Exemplo |
|---------|---------|------|-------------|---------|---------|
| $a$ | — | fase a | fase a do motor trifásico | — | [3motor](exemplos/3motor.md) |
| $D$ | — | ciclo de trabalho | fração do período em que a saída fica em nível alto | — | [555dutycycle](exemplos/555dutycycle.md) |
| $f_r$ | — | frequência de ondulação | frequência da ondulação (*ripple*) da tensão retificada | Hz | [fullrectf](exemplos/fullrectf.md) |
| $I_{\text{bat}}$ | — | corrente da bateria | corrente que a bateria entrega ao circuito | A | [joule-thief](exemplos/joule-thief.md) |
| $I_{\text{saída}}$ | — | corrente de saída | corrente que o conversor entrega à saída | A | [conv-boost](exemplos/conv-boost.md) |
| $N$ | — | número de estágios | quantidade de estágios iguais ligados em cascata | — | [spark-marx](exemplos/spark-marx.md) |
| $P_{\text{entrada}}$ | — | potência de entrada | potência que a fonte entrega ao circuito | W | [conv-boost](exemplos/conv-boost.md) |
| $P_{\text{saida}}$ | — | potência de saída | potência que chega à carga | W | [joule-thief](exemplos/joule-thief.md) |
| $P_{\text{saída}}$ | — | potência de saída | potência que chega à carga | W | [conv-boost](exemplos/conv-boost.md) |
| $V_{\text{bat}}$ | — | tensão da bateria | tensão da bateria que alimenta o circuito | V | [joule-thief](exemplos/joule-thief.md) |
| $\eta$ | — | rendimento | razão entre a potência de saída e a de entrada | — | [joule-thief](exemplos/joule-thief.md) |

## Linhas de transmissão

| Símbolo | Grafias | Nome | Significado | Unidade | Exemplo |
|---------|---------|------|-------------|---------|---------|
| $V_{L,\infty}$ | — | tensão final na carga | valor da tensão na carga depois do transitório | V | [tllight](exemplos/tllight.md) |
| $V_{\text{lanç}}$ | — | tensão lançada | tensão do pulso que entra na linha | V | [tllight](exemplos/tllight.md) |
| $Z_0$ | — | impedância característica | razão entre tensão e corrente de uma onda que percorre a linha | Ω | [ladder](exemplos/ladder.md) |
| $Z_{\text{aberta}}$ | — | impedância em aberto | impedância de entrada da linha terminada em aberto | Ω | [tlfreq](exemplos/tlfreq.md) |
| $Z_{\text{curto}}$ | — | impedância em curto | impedância de entrada da linha terminada em curto-circuito | Ω | [tlfreq](exemplos/tlfreq.md) |

## Outros

| Símbolo | Grafias | Nome | Significado | Unidade | Exemplo |
|---------|---------|------|-------------|---------|---------|
| $A$ | — | amplitude do sinal x | valor de pico da senoide aplicada ao eixo $x$ | V | [lissa](exemplos/lissa.md) |
| $A$ | — | parâmetro de amortecimento | coeficiente do termo $\ddot x$ no sistema de jerk | — | [jerk](exemplos/jerk.md) |
| $a$ | — | entrada a da expressão | variável a da expressão que define a saída da fonte controlada (tensão ou corrente de um terminal de controle) | — | [cs-integrator](exemplos/cs-integrator.md) |
| $a$ | — | parâmetro a do sistema de Rössler | constante do sistema de equações de Rössler que define o regime caótico | — | [rossler](exemplos/rossler.md) |
| $B$ | — | amplitude do sinal y | valor de pico da senoide aplicada ao eixo $y$ | V | [lissa](exemplos/lissa.md) |
| $B$ | — | brilho | brilho do ponto desenhado na tela bidimensional | — | [plot2d-checker](exemplos/plot2d-checker.md) |
| $b$ | — | entrada b da expressão | variável b da expressão que define a saída da fonte controlada (tensão ou corrente de um terminal de controle) | — | [cs-integrator](exemplos/cs-integrator.md) |
| $b$ | — | parâmetro b do sistema de Rössler | constante do sistema de equações de Rössler que define o regime caótico | — | [rossler](exemplos/rossler.md) |
| $B_p$ | — | ponto de quebra | tensão em que a inclinação do resistor de Chua muda | V | [chua](exemplos/chua.md) |
| $B_r$ | — | brilho | brilho do ponto na tela bidimensional | — | [plot2d-color](exemplos/plot2d-color.md) |
| $c$ | — | entrada c da expressão | variável c da expressão que define a saída da fonte controlada (tensão ou corrente de um terminal de controle) | — | [cs-integrator](exemplos/cs-integrator.md) |
| $c$ | — | limite superior c | limite superior da faixa a que a função $\text{clamp}$ restringe a saída | V | [cs-opamprail](exemplos/cs-opamprail.md) |
| $c$ | — | parâmetro c do sistema de Rössler | constante do sistema de equações de Rössler que define o regime caótico | — | [rossler](exemplos/rossler.md) |
| $C_{\text{th}}$ | — | capacitância térmica | calor necessário para elevar a temperatura do filamento de um grau | J/K | [lightbulb](exemplos/lightbulb.md) |
| $D$ | — | espessura total do filme | espessura $D$ do filme fino do memristor | m | [mr](exemplos/mr.md) |
| $G$ | — | função não linear | função $G(x)$ do circuito, que produz o comportamento caótico | — | [jerk](exemplos/jerk.md) |
| $g$ | — | característica do resistor de Chua | função $g(v)$ que dá a corrente do resistor não linear em função da tensão | A | [chua](exemplos/chua.md) |
| $I_{ABC}$ | — | corrente de controle | corrente que controla o ganho do amplificador de transcondutância | A | [ota-gain](exemplos/ota-gain.md) |
| $I_{\text{hold}}$ | — | corrente de manutenção | corrente abaixo da qual o dispositivo deixa de conduzir | A | [spark-sawtooth](exemplos/spark-sawtooth.md) |
| $I_{\text{trig}}$ | — | corrente de disparo | corrente de porta que dispara o tiristor | A | [scractrig](exemplos/scractrig.md) |
| $I_X$ | — | corrente em X | corrente que entra no terminal $X$ do conversor de corrente | A | [cc2](exemplos/cc2.md) |
| $I_Y$ | — | corrente em Y | corrente que entra no terminal $Y$ (nula no conversor ideal) | A | [cc2](exemplos/cc2.md) |
| $I_Z$ | — | corrente em Z | corrente que sai do terminal $Z$, igual a $\pm I_X$ | A | [cc2](exemplos/cc2.md) |
| $J$ | — | função de jerk | função que dá a terceira derivada de $x$ em termos de $x$, $\dot x$ e $\ddot x$ | — | [jerk](exemplos/jerk.md) |
| $k$ | — | constante de deriva do memristor | constante que relaciona a velocidade da fronteira dopada à corrente, $dx/dt = k\thinspace i$ | 1/(A·s) | [mr](exemplos/mr.md) |
| $k$ | — | índice | número inteiro que numera os termos de um somatório, os bits ou os elementos de um conjunto | — | [7segdecoder](exemplos/7segdecoder.md) |
| $K_{g1}$ | — | parâmetro de corrente do triodo | constante do modelo de corrente de placa | — | [triode](exemplos/triode.md) |
| $L_{\text{sim}}$ | — | indutância simulada | indutância que o circuito ativo faz parecer ter | H | [ccinductor](exemplos/ccinductor.md) |
| $M$ | — | amplitude da modulante | valor de pico do sinal modulante $m(t)$ | V | [ota-ringmod](exemplos/ota-ringmod.md) |
| $m$ | — | índice do modo | número que identifica o modo de oscilação | — | [coupled3](exemplos/coupled3.md) |
| $m_0$ | — | inclinação interna de g | inclinação (negativa) do resistor de Chua perto da origem | S | [chua](exemplos/chua.md) |
| $m_1$ | — | inclinação externa de g | inclinação (negativa) do resistor de Chua depois dos pontos de quebra | S | [chua](exemplos/chua.md) |
| $n$ | — | número inteiro n | quantidade ou índice inteiro definido no texto (ordem do filtro, número de bits, de nós ou de espiras) | — | [actbutterlo](exemplos/actbutterlo.md) |
| $p$ | — | índice p | índice inteiro do somatório ou do harmônico | — | [3phasewye](exemplos/3phasewye.md) |
| $r$ | — | variável r | variável de controle $r$ da expressão que define o resistor controlado | — | [cs-varyresistor](exemplos/cs-varyresistor.md) |
| $R_{\text{gate}}$ | — | resistência de porta | resistor em série com a porta do tiristor | Ω | [scractrig](exemplos/scractrig.md) |
| $R_m$ | — | transresistência | razão entre a tensão de saída e a corrente de entrada | Ω | [cciamp](exemplos/cciamp.md) |
| $r_n$ | — | n-ésimo valor de bifurcação | valor do parâmetro de controle em que ocorre a $n$-ésima duplicação de período | V | [chaos2](exemplos/chaos2.md) |
| $r_{n+1}$ | — | (n+1)-ésimo valor de bifurcação | valor do parâmetro de controle na duplicação seguinte | V | [chaos2](exemplos/chaos2.md) |
| $r_{n-1}$ | — | (n−1)-ésimo valor de bifurcação | valor do parâmetro de controle na duplicação anterior | V | [chaos2](exemplos/chaos2.md) |
| $r_p$ | — | resistência de placa | resistência incremental da placa do triodo | Ω | [triodeamp](exemplos/triodeamp.md) |
| $R_X$ | — | resistência em X | resistor ligado ao terminal $X$ | Ω | [cciamp](exemplos/cciamp.md) |
| $R_Z$ | — | resistência em Z | resistor ligado ao terminal $Z$ | Ω | [cc2](exemplos/cc2.md) |
| $T$ | — | temperatura do filamento | temperatura do filamento da lâmpada | K | [lightbulb](exemplos/lightbulb.md) |
| $t_c$ | — | tempo crítico | tempo em que a resistência chega ao limite | s | [mr](exemplos/mr.md) |
| $t_d$ | — | tempo de descarga | tempo que o capacitor leva para descarregar até a tensão de vale | s | [spark-sawtooth](exemplos/spark-sawtooth.md) |
| $t_{\text{subida}}$ | — | tempo de subida | duração da rampa de subida | s | [norton-saw](exemplos/norton-saw.md) |
| $V_{\text{ctrl}}$ | — | tensão de controle | tensão que ajusta a corrente de controle | V | [ota-vca](exemplos/ota-vca.md) |
| $V_{GK}$ | — | tensão grade-cátodo | tensão entre a grade e o cátodo do triodo | V | [triode](exemplos/triode.md) |
| $V_K$ | — | tensão do cátodo | tensão do cátodo em relação à terra | V | [triodeamp](exemplos/triodeamp.md) |
| $V_{\text{pino}}$ | — | tensão do pino | tensão no pino de controle do amplificador | V | [ota-vca](exemplos/ota-vca.md) |
| $V_{PK}$ | — | tensão placa-cátodo | tensão entre a placa e o cátodo do triodo | V | [triode](exemplos/triode.md) |
| $v_r$ | — | variação relativa | variação relativa do resistor controlado | — | [cs-varyresistor](exemplos/cs-varyresistor.md) |
| $V_X$ | — | tensão em X | tensão do terminal $X$, que segue $V_Y$ | V | [cc2](exemplos/cc2.md) |
| $V_Y$ | — | tensão em Y | tensão do terminal $Y$ do conversor de corrente | V | [cc2](exemplos/cc2.md) |
| $v_Z$ | — | tensão em Z | tensão do terminal $Z$ | V | [ccvccs](exemplos/ccvccs.md) |
| $w$ | — | largura da região dopada | largura da região dopada do memristor | m | [mr](exemplos/mr.md) |
| $X$ | — | coordenada X | coordenada horizontal do ponto desenhado na tela bidimensional | — | [plot2d-checker](exemplos/plot2d-checker.md) |
| $X$ | — | terminal X | entrada de corrente do conversor de corrente, que copia a tensão de $Y$ | — | [cc2](exemplos/cc2.md) |
| $x$ | — | estado x do memristor | $w/D$: fração da espessura que está dopada (de $0$ a $1$) | — | [mr](exemplos/mr.md) |
| $x$ | — | sinal x | sinal aplicado ao eixo $x$ do osciloscópio | V | [lissa](exemplos/lissa.md) |
| $x$ | — | variável de entrada x | argumento $x$ da função que define a saída da fonte controlada | — | [cs-opamprail](exemplos/cs-opamprail.md) |
| $x$ | — | variável de estado x | uma das três variáveis do sistema de Rössler, representada por uma tensão do circuito | V | [rossler](exemplos/rossler.md) |
| $x$ | — | variável do circuito | tensão do circuito, cuja terceira derivada é dada por $J$ | V | [jerk](exemplos/jerk.md) |
| $x_1$ | — | abscissa do ponto 1 | valor de $x$ do primeiro ponto da função linear por partes | — | [cs-ramp](exemplos/cs-ramp.md) |
| $x_2$ | — | abscissa do ponto 2 | valor de $x$ do segundo ponto da função linear por partes | — | [cs-ramp](exemplos/cs-ramp.md) |
| $x_k$ | — | abscissa do ponto k | valor de $x$ do ponto de índice $k$ | — | [cs-ramp](exemplos/cs-ramp.md) |
| $Y$ | — | coordenada Y | coordenada vertical do ponto desenhado na tela bidimensional | — | [plot2d-checker](exemplos/plot2d-checker.md) |
| $Y$ | — | terminal Y | entrada de tensão do conversor de corrente (corrente nula no ideal) | — | [cc2](exemplos/cc2.md) |
| $y$ | — | coordenada y | valor do sinal aplicado ao eixo $y$ | V | [lissa](exemplos/lissa.md) |
| $y$ | — | variável de estado y | uma das três variáveis do sistema de Rössler, representada por uma tensão do circuito | V | [rossler](exemplos/rossler.md) |
| $y_1$ | — | ordenada do ponto 1 | valor de $y$ do primeiro ponto da função linear por partes | — | [cs-ramp](exemplos/cs-ramp.md) |
| $y_2$ | — | ordenada do ponto 2 | valor de $y$ do segundo ponto da função linear por partes | — | [cs-ramp](exemplos/cs-ramp.md) |
| $y_k$ | — | ordenada do ponto k | valor de $y$ do ponto de índice $k$ | — | [cs-ramp](exemplos/cs-ramp.md) |
| $z$ | — | variável de estado z | uma das três variáveis do sistema de Rössler, representada por uma tensão do circuito | V | [rossler](exemplos/rossler.md) |
| $\delta$ | — | constante de Feigenbaum | razão limite entre intervalos sucessivos de duplicação de período, cerca de $4{,}669$ | — | [chaos2](exemplos/chaos2.md) |
| $\lambda_k$ | — | autovalor k | autovalor de índice $k$ da matriz do circuito acoplado | — | [coupled1](exemplos/coupled1.md) |
| $\mu$ | — | mobilidade | mobilidade dos íons no memristor | m²/(s·V) | [mr](exemplos/mr.md) |
| $\varphi_0$ | — | fluxo inicial | valor inicial do fluxo magnético do memristor | Wb | [mr](exemplos/mr.md) |
| $\varphi_{\text{max}}$ | — | fluxo máximo | fluxo em que a resistência chega ao valor mínimo | Wb | [mr-sine2](exemplos/mr-sine2.md) |

## Campos dos componentes

Os campos abaixo aparecem nas tabelas de componentes de cada exemplo (`Campo = valor`). Os valores com sufixo seguem a notação do simulador: `k` é $10^3$, `M` é $10^6$, `m` é $10^{-3}$, `u` é $10^{-6}$, `n` é $10^{-9}$ e `p` é $10^{-12}$; uma tabela só mostra o número, e a unidade é a da coluna *Unidade* de cada campo.

### Fontes e terra

| Campo | Componentes | Unidade | Efeito | Exemplo |
|-------|-------------|---------|--------|---------|
| Circle Symbol | `VoltageElm` | sim/não | Desenha a fonte como círculo em vez de linhas; só o desenho muda. | [cap](exemplos/cap.md) |
| Current (A) | `CurrentElm` | A | Corrente que a fonte de corrente força no ramo, qualquer que seja a tensão sobre ela. | [cciamp](exemplos/cciamp.md) |
| DC Offset (V) | `VoltageElm`, `RailElm`, `DCVoltageElm` | V | Nível contínuo somado à forma de onda; desloca a onda para cima ou para baixo sem mudar a amplitude. | [3-cgand](exemplos/3-cgand.md) |
| Duty Cycle | `VoltageElm`, `RailElm`, `DCVoltageElm` | — (fração de 0 a 1) | Razão cíclica: fração do período em que a onda quadrada ou o pulso fica no nível alto. | [555missing](exemplos/555missing.md) |
| Frequency (Hz) | `VoltageElm`, `RailElm`, `DCVoltageElm` | Hz | Frequência da forma de onda: quantos ciclos completos por segundo. | [3motor](exemplos/3motor.md) |
| Internal Resistance (ohms) | `VoltageElm`, `RailElm`, `DCVoltageElm` | Ω | Resistência em série com a fonte; $0$ é uma fonte ideal, sem queda de tensão sob carga. | [3-cgand](exemplos/3-cgand.md) |
| Max Voltage | `VoltageElm`, `RailElm`, `VarRailElm` | V | Amplitude da fonte: o valor de pico da forma de onda (o próprio valor, numa fonte contínua). | [3motor](exemplos/3motor.md) |
| Max Voltage (V, 0=unlimited) | `CurrentElm` | V | Limite da tensão sobre a fonte de corrente; $0$ significa sem limite. | [cciamp](exemplos/cciamp.md) |
| Min Voltage | `VarRailElm` | V | Tensão mínima do *slider*: o valor da fonte variável quando o cursor está em uma extremidade. | [cc2](exemplos/cc2.md) |
| Phase Offset (degrees) | `VoltageElm`, `RailElm`, `DCVoltageElm` | grau | Defasagem inicial da onda: desloca o instante em que o ciclo começa ($360^\circ$ é um ciclo inteiro). | [3motor](exemplos/3motor.md) |
| Rise/Fall Time (s) | `VoltageElm`, `RailElm`, `DCVoltageElm` | s | Tempo que as bordas de subida e de descida da onda levam para completar a transição. | [555missing](exemplos/555missing.md) |
| Show Voltage | `VoltageElm`, `RailElm`, `OutputElm` | sim/não | Mostra o valor da tensão ao lado do elemento no esquema; não altera o circuito. | [3motor](exemplos/3motor.md) |
| Slider Text | `VarRailElm`, `PotElm` | — | Texto do rótulo do *slider* que ajusta o elemento. | [555dutycycle](exemplos/555dutycycle.md) |
| Specify As | `VoltageElm`, `RailElm` | — | Como a amplitude é dada: pelo valor de pico ou pelos níveis (máximo e mínimo) da onda. | [555missing](exemplos/555missing.md) |
| Symbol | `GroundElm` | — | Desenho do símbolo de terra (terra, chassi, sinal); o nó é sempre $0\ \text{V}$. | [3-cgand](exemplos/3-cgand.md) |
| Voltage | `VoltageElm`, `RailElm` | V | Tensão da fonte quando ela é especificada diretamente (fonte contínua ou valor inicial do *slider*). | [3-cgand](exemplos/3-cgand.md) |
| Waveform | `VoltageElm`, `RailElm` | — | Forma de onda da fonte: contínua (D/C), senoidal (A/C), quadrada, triangular, dente de serra, pulso, ruído ou variável (controlada por *slider*). | [3-cgand](exemplos/3-cgand.md) |

### Resistores, capacitores e indutores

| Campo | Componentes | Unidade | Efeito | Exemplo |
|-------|-------------|---------|--------|---------|
| Capacitance (F) | `CapacitorElm`, `PolarCapacitorElm` | F | Capacitância: quanta carga o capacitor guarda por volt. | [555dutycycle](exemplos/555dutycycle.md) |
| Coil Resistance (ohms) | `RelayCoilElm` | Ω | Resistência do fio da bobina do relé. | [latchingrelay](exemplos/latchingrelay.md) |
| Coupling Coefficient | `TransformerElm`, `TappedTransformerElm`, `ThreePhaseMotorElm` | — (0 a 1) | Acoplamento magnético entre os enrolamentos; $1$ é o acoplamento perfeito. | [3motor](exemplos/3motor.md) |
| Delay (s) | `TransLineElm` | s | Tempo que um sinal leva para percorrer a linha de transmissão. | [tl](exemplos/tl.md) |
| Friction coefficient (Nms/rad) | `ThreePhaseMotorElm` | N·m·s/rad | Atrito viscoso: conjugado de freio proporcional à velocidade. | [3motor](exemplos/3motor.md) |
| Gyration Resistance (Ω) | `GyratorElm` | Ω | Resistência de giração: converte a impedância de uma porta na recíproca, vista na outra. | [gyratorelm](exemplos/gyratorelm.md) |
| Impedance (ohms) | `TransLineElm` | Ω | Impedância característica da linha de transmissão. | [tl](exemplos/tl.md) |
| Inductance (H) | `InductorElm`, `CrystalElm`, `RelayCoilElm` | H | Indutância: quanto fluxo magnético a bobina cria por ampère. | [amdetect](exemplos/amdetect.md) |
| Initial Current (on Reset) (A) | `InductorElm` | A | Corrente do indutor quando a simulação é reiniciada. | [amdetect](exemplos/amdetect.md) |
| Initial Voltage (on Reset) | `CapacitorElm`, `PolarCapacitorElm` | V | Tensão do capacitor quando a simulação é reiniciada. | [555dutycycle](exemplos/555dutycycle.md) |
| Max Resistance (ohms) | `MemristorElm` | Ω | Resistência do memristor com a região dopada vazia ($R_\text{off}$). | [mr](exemplos/mr.md) |
| Max Reverse Voltage | `PolarCapacitorElm` | V | Tensão reversa máxima que o capacitor polarizado suporta antes de se danificar. | [conv-boost](exemplos/conv-boost.md) |
| Min Resistance (ohms) | `MemristorElm` | Ω | Resistência do memristor com a região dopada completa ($R_\text{on}$). | [mr](exemplos/mr.md) |
| Mobility (um^2/(s*V)) | `MemristorElm` | µm²/(s·V) | Mobilidade dos íons: quão rápido a região dopada se move com a corrente. | [mr](exemplos/mr.md) |
| Moment of inertia (Kg.m^2) | `ThreePhaseMotorElm` | kg·m² | Inércia do rotor e da carga: quanto o motor resiste a mudar de velocidade. | [3motor](exemplos/3motor.md) |
| Off Current (A) | `RelayCoilElm` | A | Corrente da bobina abaixo da qual o relé abre. | [latchingrelay](exemplos/latchingrelay.md) |
| On Current (A) | `RelayCoilElm` | A | Corrente da bobina acima da qual o relé fecha. | [latchingrelay](exemplos/latchingrelay.md) |
| Parallel Capacitance | `CrystalElm` | F | Capacitância paralela $C_0$ do cristal (a dos eletrodos e do encapsulamento). | [crystalosc](exemplos/crystalosc.md) |
| Primary Inductance (H) | `TransformerElm`, `TappedTransformerElm` | H | Indutância do enrolamento primário do transformador. | [joule-thief](exemplos/joule-thief.md) |
| Ratio (N1/N2) | `TransformerElm`, `TappedTransformerElm` | — | Relação de espiras primário/secundário: a tensão do secundário é a do primário dividida por ela. | [joule-thief](exemplos/joule-thief.md) |
| Resistance (ohms) | `ResistorElm`, `PotElm` | Ω | Resistência do componente; num potenciômetro, a resistência total entre as extremidades. | [3phasewye](exemplos/3phasewye.md) |
| Resistance (Ω) | `CrystalElm` | Ω | Resistência em série do ramo do cristal (as perdas do ressonador). | [crystalosc](exemplos/crystalosc.md) |
| Rotor Inductance (H) | `ThreePhaseMotorElm` | H | Indutância de dispersão do rotor. | [3motor](exemplos/3motor.md) |
| Rotor Resistance (ohms) | `ThreePhaseMotorElm` | Ω | Resistência do rotor do motor trifásico. | [3motor](exemplos/3motor.md) |
| Saturation Current (A) (0=none) | `InductorElm`, `TransformerElm` | A | Corrente em que o núcleo satura e a indutância cai; $0$ desliga a saturação. | [amdetect](exemplos/amdetect.md) |
| Series Capacitance (F) | `CrystalElm` | F | Capacitância em série $C_s$ do ramo ressonante do cristal. | [crystalosc](exemplos/crystalosc.md) |
| Series Resistance | `CapacitorElm`, `PolarCapacitorElm` | Ω | Resistência em série equivalente do capacitor (ESR); $0$ é um capacitor ideal. | [555dutycycle](exemplos/555dutycycle.md) |
| Show Frequency | `CrystalElm` | sim/não | Mostra a frequência de ressonância do cristal ao lado do elemento. | [crystalosc](exemplos/crystalosc.md) |
| Stator Inductance (H) | `ThreePhaseMotorElm` | H | Indutância de dispersão do estator. | [3motor](exemplos/3motor.md) |
| Stator Resistance (ohms) | `ThreePhaseMotorElm` | Ω | Resistência de cada enrolamento do estator do motor trifásico. | [3motor](exemplos/3motor.md) |
| Swap Secondary Polarity | `TransformerElm` | sim/não | Inverte a polaridade do secundário (inverte os pontos de enrolamento). | [joule-thief](exemplos/joule-thief.md) |
| Switching Time (s) | `RelayCoilElm` | s | Tempo que o relé leva para mudar de estado. | [latchingrelay](exemplos/latchingrelay.md) |
| Total Width (nm) | `MemristorElm` | nm | Espessura total do filme do memristor. | [mr](exemplos/mr.md) |
| Trapezoidal Approximation | `CapacitorElm`, `InductorElm`, `TransformerElm` | sim/não | Escolhe o método de integração no tempo do elemento: trapezoidal (mais exato) ou Euler para trás (mais amortecido). | [555dutycycle](exemplos/555dutycycle.md) |
| Type | `RelayCoilElm` | — | Tipo do relé (por exemplo, de contato único ou com travamento). | [latchingrelay](exemplos/latchingrelay.md) |
| Width of Doped Region (nm) | `MemristorElm` | nm | Largura inicial da região dopada, que define o estado do memristor. | [mr](exemplos/mr.md) |

### Chaves e relés

| Campo | Componentes | Unidade | Efeito | Exemplo |
|-------|-------------|---------|--------|---------|
| # of Throws | `Switch2Elm` | — | Número de posições (vias) da chave seletora. | [3way](exemplos/3way.md) |
| Breakdown voltage | `SparkGapElm` | V | Tensão em que o centelhador começa a conduzir. | [spark-marx](exemplos/spark-marx.md) |
| Breakdown voltage (volts) | `DiacElm` | V | Tensão de disparo do diac. | [triacdimmer](exemplos/triacdimmer.md) |
| Group Number (for linking) | `Switch2Elm`, `PotElm` | — | Número de grupo: elementos do mesmo grupo se movem juntos. | [3way](exemplos/3way.md) |
| Hold current (amps) | `DiacElm` | A | Corrente mínima para o diac continuar conduzindo. | [triacdimmer](exemplos/triacdimmer.md) |
| Holding current (A) | `SparkGapElm` | A | Corrente mínima para o centelhador continuar conduzindo. | [spark-marx](exemplos/spark-marx.md) |
| IEC Symbol | `SwitchElm`, `RelayContactElm` | sim/não | Desenha a chave com o símbolo da norma IEC; só o desenho muda. | [3phasewye](exemplos/3phasewye.md) |
| Keyboard Shortcut | `LogicInputElm`, `SwitchElm`, `Switch2Elm` | — | Tecla que aciona a chave com o teclado. | [3-cgand](exemplos/3-cgand.md) |
| Label (for linking) | `SwitchElm`, `RelayCoilElm`, `RelayContactElm` | — | Rótulo que liga elementos: chaves com o mesmo rótulo mudam juntas, e o contato segue sua bobina. | [3phasewye](exemplos/3phasewye.md) |
| Momentary Switch | `SwitchElm`, `Switch2Elm`, `LogicInputElm` | sim/não | A chave só fica fechada enquanto é pressionada e volta sozinha ao soltar. | [3-cgand](exemplos/3-cgand.md) |
| Normally closed | `AnalogSwitchElm` | sim/não | A chave analógica é fechada quando o controle está em nível baixo. | [cmosmsff](exemplos/cmosmsff.md) |
| Normally Closed | `RelayContactElm` | sim/não | O contato do relé é fechado em repouso e abre quando a bobina é energizada. | [latchingrelay](exemplos/latchingrelay.md) |
| Off Resistance (ohms) | `AnalogSwitchElm`, `AnalogSwitch2Elm`, `RelayContactElm` | Ω | Resistência da chave aberta; é grande, mas finita. | [cmosmsff](exemplos/cmosmsff.md) |
| Off resistance (ohms) | `SparkGapElm`, `DiacElm` | Ω | Resistência do dispositivo antes de disparar. | [spark-marx](exemplos/spark-marx.md) |
| On Resistance (ohms) | `SwitchElm`, `Switch2Elm`, `AnalogSwitch2Elm` | Ω | Resistência da chave fechada. | [3phasewye](exemplos/3phasewye.md) |
| On resistance (ohms) | `SparkGapElm`, `DiacElm` | Ω | Resistência do dispositivo depois de disparar. | [spark-marx](exemplos/spark-marx.md) |
| Pulldown Resistor | `AnalogSwitchElm`, `AnalogSwitch2Elm` | sim/não | Liga um resistor da saída à terra, para que ela não fique flutuando quando a chave abre. | [cmosmsff](exemplos/cmosmsff.md) |
| Swap NO/NC | `AnalogSwitch2Elm` | sim/não | Troca os contatos normalmente aberto e normalmente fechado da chave de duas vias. | [deltasigma](exemplos/deltasigma.md) |
| Threshold | `LogicOutputElm`, `AnalogSwitchElm`, `AnalogSwitch2Elm` | V | Tensão de limiar: acima dela o sinal é lido como alto (ou a chave fecha). | [3-cgand](exemplos/3-cgand.md) |

### Semicondutores

| Campo | Componentes | Unidade | Efeito | Exemplo |
|-------|-------------|---------|--------|---------|
| Beta/hFE | `NTransistorElm`, `PTransistorElm` | — | Ganho de corrente $\beta$ do transistor bipolar: relação entre as correntes de coletor e de base. | [555int](exemplos/555int.md) |
| Capacitance @ 0V (F) | `VaractorElm` | F | Capacitância do varactor sem tensão reversa ($C_{j0}$). | [varactor](exemplos/varactor.md) |
| Color | `LEDElm`, `TextElm` | — | Cor do LED (muda a tensão de condução e o brilho) ou do texto. | [3phasewye](exemplos/3phasewye.md) |
| Draw Circle | `NTransistorElm`, `PTransistorElm` | sim/não | Desenha o círculo em volta do transistor; só o desenho muda. | [555int](exemplos/555int.md) |
| Gate Resistance (ohms) | `SCRElm` | Ω | Resistência entre a porta e o cátodo do SCR. | [scr](exemplos/scr.md) |
| Gate-MT1 Resistance (ohms) | `TriacElm` | Ω | Resistência entre a porta e o terminal MT1 do triac. | [triacdimmer](exemplos/triacdimmer.md) |
| Holding Current (A) | `SCRElm`, `TriacElm` | A | Corrente mínima de manutenção: abaixo dela o tiristor desliga. | [scr](exemplos/scr.md) |
| kg1 | `TriodeElm` | — | Parâmetro de ganho do triodo a vácuo no modelo de corrente de placa. | [triode](exemplos/triode.md) |
| Max Brightness Current (A) | `LEDElm` | A | Corrente em que o LED atinge o brilho máximo desenhado. | [joule-thief](exemplos/joule-thief.md) |
| Model | `DiodeElm`, `NTransistorElm`, `NMosfetElm` | — | Conjunto de parâmetros do dispositivo (por exemplo, o diodo 1N4004 ou um MOSFET de limiar $V_t$ dado). | [3-cgand](exemplos/3-cgand.md) |
| Model Name | `CustomCompositeElm`, `CustomLogicElm` | — | Nome do modelo do subcircuito ou da lógica personalizada. | [adder4-sc](exemplos/adder4-sc.md) |
| mu | `TriodeElm` | — | Fator de amplificação $\mu$ do triodo: quanto a grade controla a placa. | [triode](exemplos/triode.md) |
| Swap D/S | `NMosfetElm`, `PMosfetElm`, `NJfetElm` | sim/não | Troca os terminais de dreno e fonte no esquema. | [3-cgand](exemplos/3-cgand.md) |
| Swap E/C | `NTransistorElm`, `PTransistorElm` | sim/não | Troca os terminais de emissor e coletor no esquema. | [555int](exemplos/555int.md) |
| Trigger Current (A) | `SCRElm`, `TriacElm` | A | Corrente de porta necessária para disparar o tiristor. | [scr](exemplos/scr.md) |

### Amplificadores e fontes controladas

| Campo | Componentes | Unidade | Efeito | Exemplo |
|-------|-------------|---------|--------|---------|
| Bidirectional | `SweepElm` | sim/não | A varredura sobe e depois desce, em vez de recomeçar. | [allpass1](exemplos/allpass1.md) |
| Cooldown Time (s) | `LampElm` | s | Tempo que o filamento da lâmpada leva para esfriar. | [lightbulb](exemplos/lightbulb.md) |
| Gain | `OpAmpElm` | — | Ganho de malha aberta do amplificador operacional. | [555int](exemplos/555int.md) |
| Logarithmic | `SweepElm` | sim/não | A frequência varia em escala logarítmica em vez de linear. | [allpass1](exemplos/allpass1.md) |
| Low Voltage (V) | `SchmittElm`, `InvertingSchmittElm` | V | Tensão da saída no nível baixo. | [delta-pwm](exemplos/delta-pwm.md) |
| Lower threshold (V) | `SchmittElm`, `InvertingSchmittElm` | V | Limiar inferior do gatilho Schmitt: a saída muda quando a entrada o ultrapassa descendo. | [delta-pwm](exemplos/delta-pwm.md) |
| Max Frequency (Hz) | `SweepElm` | Hz | Frequência final da varredura. | [allpass1](exemplos/allpass1.md) |
| Max Output (V) | `OpAmpElm` | V | Tensão de saturação positiva (o trilho de alimentação) da saída. | [555int](exemplos/555int.md) |
| Min Frequency (Hz) | `SweepElm` | Hz | Frequência inicial da varredura. | [allpass1](exemplos/allpass1.md) |
| Min Output (V) | `OpAmpElm` | V | Tensão de saturação negativa da saída. | [555int](exemplos/555int.md) |
| Negative Supply Voltage (V) | `OTAElm` | V | Alimentação negativa do amplificador de transcondutância. | [ota-gain](exemplos/ota-gain.md) |
| Nominal Power | `LampElm` | W | Potência nominal da lâmpada. | [lightbulb](exemplos/lightbulb.md) |
| Nominal Voltage | `LampElm` | V | Tensão nominal da lâmpada. | [lightbulb](exemplos/lightbulb.md) |
| Output Current | `ControlledCurrentElm` | A | Corrente de saída da fonte controlada. | [cs-currentadder](exemplos/cs-currentadder.md) |
| Output Function | `VCVSElm`, `VCCSElm` | — | Expressão que dá a saída da fonte controlada em função das entradas. | [cs-integrator](exemplos/cs-integrator.md) |
| Output Voltage | `ControlledVoltageElm` | V | Tensão de saída da fonte controlada. | [cs-diff](exemplos/cs-diff.md) |
| Positive Supply Voltage (5-20V) | `OTAElm` | V | Alimentação positiva do amplificador de transcondutância. | [ota-gain](exemplos/ota-gain.md) |
| Slew Rate (V/ns) | `InverterElm`, `SchmittElm`, `InvertingSchmittElm` | V/ns | Taxa máxima de variação da saída: limita a rapidez das bordas. | [555int](exemplos/555int.md) |
| Sweep Time (s) | `SweepElm` | s | Duração de uma varredura de frequência. | [allpass1](exemplos/allpass1.md) |
| Upper threshold (V) | `SchmittElm`, `InvertingSchmittElm` | V | Limiar superior do gatilho Schmitt: a saída muda quando a entrada o ultrapassa subindo. | [delta-pwm](exemplos/delta-pwm.md) |
| Warmup Time (s) | `LampElm` | s | Tempo que o filamento da lâmpada leva para esquentar. | [lightbulb](exemplos/lightbulb.md) |

### Lógica e digital

| Campo | Componentes | Unidade | Efeito | Exemplo |
|-------|-------------|---------|--------|---------|
| # of Address Bits | `ROMElm` | — | Número de bits de endereço da memória. | [plot2d-smile](exemplos/plot2d-smile.md) |
| # of Bits | `BusSplitterElm`, `Counter2Elm`, `DecimalDisplayElm` | — | Largura em bits do elemento. | [alu74181](exemplos/alu74181.md) |
| # of Data Bits | `ROMElm` | — | Número de bits de dado da memória. | [plot2d-smile](exemplos/plot2d-smile.md) |
| # of Inputs | `AndGateElm`, `NandGateElm`, `XorGateElm` | — | Número de entradas da porta lógica. | [555int](exemplos/555int.md) |
| # of Select Bits | `MultiplexerElm`, `DeMultiplexerElm` | — | Número de bits de seleção do multiplexador. | [ledarray](exemplos/ledarray.md) |
| Bit Order | `Counter2Elm`, `DecimalDisplayElm`, `ADCElm` | — | Ordem dos bits: o mais significativo primeiro ou por último. | [7segdecoder](exemplos/7segdecoder.md) |
| Bus Select | `MultiplexerElm`, `DeMultiplexerElm` | sim/não | O seletor é um único pino de barramento em vez de pinos separados. | [ledarray](exemplos/ledarray.md) |
| Bus Width | `InstructionDisplayElm`, `BusLogicInputElm` | — | Largura do barramento, em bits. | [alu74181](exemplos/alu74181.md) |
| Contents | `ROMElm` | — | Conteúdo inicial da memória. | [plot2d-smile](exemplos/plot2d-smile.md) |
| Current Required | `LogicOutputElm` | sim/não | A saída puxa a entrada para baixo (resistor de pulldown), exigindo corrente do circuito. | [3-cgand](exemplos/3-cgand.md) |
| Data Bus Width | `MultiplexerElm` | — | Largura, em bits, de cada entrada de dados. | [td4](exemplos/td4.md) |
| Definition | `CustomLogicElm` | — | Regras que definem as saídas em função das entradas. | [ledarray](exemplos/ledarray.md) |
| DeMorgan's Symbol | `AndGateElm`, `NandGateElm`, `InverterElm` | sim/não | Desenha a porta no símbolo equivalente de De Morgan; só o desenho muda. | [555int](exemplos/555int.md) |
| Duration (s) | `AudioOutputElm` | s | Duração do trecho de áudio gerado. | [actbutterband](exemplos/actbutterband.md) |
| Edge Triggered | `LatchElm` | sim/não | O elemento guarda o dado na borda do clock, e não enquanto o clock está alto. | [deltasigma](exemplos/deltasigma.md) |
| Enable Pins | `LatchElm` | — | Pinos de habilitação do latch. | [deltasigma](exemplos/deltasigma.md) |
| Ground Pin | `TimerElm` | sim/não | Mostra o pino de terra do temporizador 555. | [555dutycycle](exemplos/555dutycycle.md) |
| Hex Display | `ROMElm` | sim/não | Mostra os valores da memória em hexadecimal. | [plot2d-smile](exemplos/plot2d-smile.md) |
| High Logic Voltage | `LogicInputElm`, `AndGateElm`, `NandGateElm` | V | Tensão do nível lógico alto. | [3-cgand](exemplos/3-cgand.md) |
| High Logic Voltage (0=default) | `CustomCompositeElm` | V | Tensão do nível alto do bloco; $0$ usa o padrão do circuito. | [adder4-sc](exemplos/adder4-sc.md) |
| High Voltage | `BusLogicInputElm` | V | Tensão do nível alto da entrada de barramento. | [td4](exemplos/td4.md) |
| Info Text | `CustomLogicElm` | — | Texto de informação mostrado no bloco. | [ledarray](exemplos/ledarray.md) |
| Input Mode | `MultiplexerElm` | — | Modo das entradas do multiplexador (por exemplo, carga paralela). | [piso-sr](exemplos/piso-sr.md) |
| Inputs | `CustomLogicElm` | — | Nomes dos pinos de entrada da lógica personalizada. | [ledarray](exemplos/ledarray.md) |
| Invert Inputs | `AndGateElm`, `NandGateElm`, `NorGateElm` | sim/não | Inverte todas as entradas da porta (bolhas de inversão). | [555int](exemplos/555int.md) |
| Invert Reset | `JKFlipFlopElm` | sim/não | O pino de limpeza é ativo em nível baixo. | [counter](exemplos/counter.md) |
| Invert reset pin | `CounterElm`, `RingCounterElm` | sim/não | O pino de limpeza do contador é ativo em nível baixo. | [deltasigma](exemplos/deltasigma.md) |
| Invert Set/Reset | `DFlipFlopElm` | sim/não | Os pinos de ajuste e de limpeza são ativos em nível baixo. | [deltasigma](exemplos/deltasigma.md) |
| Inverted Output | `MultiplexerElm` | sim/não | Inverte a saída do multiplexador. | [piso-sr](exemplos/piso-sr.md) |
| Keep Inactive Outputs High (74139) | `DeMultiplexerElm` | sim/não | As saídas não selecionadas ficam em nível alto (como no 74139). | [ledarray](exemplos/ledarray.md) |
| Lookup Table | `InstructionDisplayElm` | — | Tabela que associa o código de cada instrução ao seu nome. | [alu74181](exemplos/alu74181.md) |
| Low Voltage | `LogicInputElm`, `BusLogicInputElm` | V | Tensão do nível lógico baixo. | [3-cgand](exemplos/3-cgand.md) |
| Modulus | `Counter2Elm`, `CounterElm` | — | Módulo do contador: ele conta de $0$ até esse valor menos um e recomeça. | [deltasigma](exemplos/deltasigma.md) |
| Negative Edge Triggered | `CounterElm` | sim/não | O contador muda na borda de descida do clock. | [deltasigma](exemplos/deltasigma.md) |
| Numeric | `LogicInputElm`, `LogicOutputElm` | sim/não | Mostra $0$ e $1$ em vez de $L$ e $H$. | [3-cgand](exemplos/3-cgand.md) |
| Output Mode | `DeMultiplexerElm` | — | Modo das saídas do demultiplexador. | [ledarray](exemplos/ledarray.md) |
| Outputs | `CustomLogicElm` | — | Nomes dos pinos de saída da lógica personalizada. | [ledarray](exemplos/ledarray.md) |
| Positive Edge Triggered | `JKFlipFlopElm` | sim/não | O flip-flop muda na borda de subida do clock. | [counter](exemplos/counter.md) |
| Propagation Delay (s) | `AndGateElm`, `NandGateElm`, `XorGateElm` | s | Atraso entre uma mudança na entrada e a resposta da saída. | [555int](exemplos/555int.md) |
| Reset Pin | `DFlipFlopElm`, `JKFlipFlopElm`, `LatchElm` | sim/não | Acrescenta o pino de limpeza (*reset*) assíncrono, que força a saída em $0$. | [counter](exemplos/counter.md) |
| Restore Contents on Reset | `ROMElm` | sim/não | Restaura o conteúdo original quando a simulação é reiniciada. | [plot2d-smile](exemplos/plot2d-smile.md) |
| Sampling Rate | `AudioOutputElm` | Hz | Taxa de amostragem do áudio reproduzido. | [actbutterband](exemplos/actbutterband.md) |
| Schmitt Inputs | `AndGateElm`, `NandGateElm`, `XorGateElm` | sim/não | As entradas da porta têm histerese (gatilho Schmitt). | [555int](exemplos/555int.md) |
| Set Pin | `DFlipFlopElm`, `LatchElm`, `TFlipFlopElm` | sim/não | Acrescenta o pino de ajuste (*set*) assíncrono, que força a saída em $1$. | [deltasigma](exemplos/deltasigma.md) |
| Show Pin Numbers | `TimerElm` | sim/não | Mostra o número de cada pino do 555 no desenho. | [555dutycycle](exemplos/555dutycycle.md) |
| Strobe Pin | `MultiplexerElm` | sim/não | Acrescenta um pino de habilitação (*strobe*). | [piso-sr](exemplos/piso-sr.md) |
| Ternary | `LogicInputElm`, `LogicOutputElm` | sim/não | Lógica de três níveis: $0$, $1$ e $2$. | [3-cgand](exemplos/3-cgand.md) |
| Threshold Voltage | `InstructionDisplayElm` | V | Tensão acima da qual a entrada do mostrador é lida como nível alto. | [alu74181](exemplos/alu74181.md) |
| Up/Down Pin | `CounterElm` | sim/não | Acrescenta um pino que escolhe contar para cima ou para baixo. | [deltasigma](exemplos/deltasigma.md) |

### Mostradores, medidores e texto

| Campo | Componentes | Unidade | Efeito | Exemplo |
|-------|-------------|---------|--------|---------|
| Blank on 1111 | `SevenSegDecoderElm` | sim/não | Apaga o mostrador quando a entrada é $1111$ (15). | [adder4-sc](exemplos/adder4-sc.md) |
| Blank Pin | `SevenSegDecoderElm` | sim/não | Acrescenta um pino que apaga o mostrador. | [adder4-sc](exemplos/adder4-sc.md) |
| Circular Symbol | `AmmeterElm` | sim/não | Desenha o amperímetro como círculo; só o desenho muda. | [cs-currentadder](exemplos/cs-currentadder.md) |
| Diodes | `SevenSegElm` | sim/não | Desenha cada segmento como um diodo emissor de luz. | [7segdecoder](exemplos/7segdecoder.md) |
| Display Mode | `DecimalDisplayElm` | — | Como o número é mostrado (decimal, com ou sem sinal, hexadecimal). | [brentkung](exemplos/brentkung.md) |
| Draw Bar On Top | `TextElm` | sim/não | Desenha uma barra sobre o texto (negação). | [3phasewye](exemplos/3phasewye.md) |
| Extra Segment | `SevenSegElm` | sim/não | Acrescenta o ponto decimal ao mostrador. | [7segdecoder](exemplos/7segdecoder.md) |
| Fixed Precision | `OutputElm` | sim/não | Mostra o valor com um número fixo de casas decimais. | [conv-buckboost](exemplos/conv-buckboost.md) |
| Grid Height | `LEDArrayElm` | — | Número de linhas da matriz de LEDs. | [ledarray](exemplos/ledarray.md) |
| Grid Width | `LEDArrayElm` | — | Número de colunas da matriz de LEDs. | [ledarray](exemplos/ledarray.md) |
| Internal Node | `LabeledNodeElm` | sim/não | O nó rotulado é interno ao bloco e não aparece fora dele. | [alu74181](exemplos/alu74181.md) |
| Label | `ProbeElm`, `AmmeterElm` | — | Texto que identifica o medidor no esquema. | [555schmitt](exemplos/555schmitt.md) |
| Rotate Text When Vertical | `LabeledNodeElm` | sim/não | Gira o texto do rótulo quando o elemento está na vertical. | [alu74181](exemplos/alu74181.md) |
| Scale | `ProbeElm`, `OutputElm`, `AmmeterElm` | — | Escala do mostrador: automática ou uma unidade fixa. | [555schmitt](exemplos/555schmitt.md) |
| Segments | `SevenSegElm`, `SevenSegDecoderElm` | — | Número de segmentos do mostrador. | [7segdecoder](exemplos/7segdecoder.md) |
| Series Resistance (0 = infinite) | `ProbeElm` | Ω | Resistência em série do medidor; $0$ o torna um voltímetro ideal, de resistência infinita. | [555schmitt](exemplos/555schmitt.md) |
| Show Bus Value | `RoutedWireElm` | sim/não | Mostra o valor do barramento sobre o fio. | [fullrect-sc](exemplos/fullrect-sc.md) |
| Show Bus Value (Hex) | `RoutedWireElm` | sim/não | Mostra o valor do barramento em hexadecimal. | [fullrect-sc](exemplos/fullrect-sc.md) |
| Show Current | `RoutedWireElm` | sim/não | Mostra a corrente que passa pelo fio. | [fullrect-sc](exemplos/fullrect-sc.md) |
| Show Values | `PotElm` | sim/não | Mostra no esquema as resistências em cada lado do cursor do potenciômetro. | [555dutycycle](exemplos/555dutycycle.md) |
| Size | `TextElm` | — | Tamanho da fonte do texto. | [3phasewye](exemplos/3phasewye.md) |
| Text | `TextElm`, `LabeledNodeElm` | — | Texto do elemento; num nó rotulado, o nome que liga pontos do circuito. | [3phasewye](exemplos/3phasewye.md) |
| Use Circle Symbol | `ProbeElm` | sim/não | Desenha o medidor como círculo; só o desenho muda. | [555schmitt](exemplos/555schmitt.md) |
| Value | `ProbeElm`, `AmmeterElm`, `BusLogicInputElm` | — | Grandeza que o medidor mostra (tensão, valor eficaz, pico, frequência...). | [555schmitt](exemplos/555schmitt.md) |
