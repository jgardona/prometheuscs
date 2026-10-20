# Guia do usuário

Este guia descreve tudo o que o prometheuscs faz e como usar. Se é a primeira vez, leia a seção
1 e depois a 2: em cinco minutos você monta e roda um circuito.

Os nomes dos menus aparecem aqui como na janela. Onde o CircuitJS1 original não tem tradução,
o menu fica em inglês (por exemplo, as categorias do menu Desenhar).

## Sumário

1. [Primeiros passos](#1-primeiros-passos)
2. [Um circuito do zero](#2-um-circuito-do-zero)
3. [A janela](#3-a-janela)
4. [Montar o circuito](#4-montar-o-circuito)
5. [Executar a simulação](#5-executar-a-simulação)
6. [Osciloscópios](#6-osciloscópios)
7. [Sliders (controles deslizantes)](#7-sliders)
8. [Arquivos, exemplos e compartilhamento](#8-arquivos-exemplos-e-compartilhamento)
9. [Subcircuitos](#9-subcircuitos)
10. [Opções](#10-opções)
11. [Componentes](#11-componentes)
12. [Áudio, dados e arquivos de amostras](#12-áudio-dados-e-arquivos-de-amostras)
13. [Atalhos de teclado e mouse](#13-atalhos-de-teclado-e-mouse)
14. [Quando algo dá errado](#14-quando-algo-dá-errado)
15. [Linha de comando e arquivos do programa](#15-linha-de-comando-e-arquivos-do-programa)
16. [Diferenças em relação ao CircuitJS1](#16-diferenças-em-relação-ao-circuitjs1)

---

## 1. Primeiros passos

Compile e abra o programa (precisa de GTK 4 e libadwaita; veja o [README](../README.md)):

```bash
cargo run --release --features gui
```

Para ver tudo funcionando de uma vez, abra o menu **Exemplos de Circuitos** e escolha, por
exemplo, **Circuito RLC**. O circuito começa a rodar sozinho: os fios mudam de cor com a tensão,
os pontos amarelos andam com a corrente e os osciloscópios, embaixo, desenham as formas de onda.

Há 373 exemplos, organizados em submenus (circuitos básicos, filtros, osciladores, lógica
digital, memórias, motores, e assim por diante). Eles são os mesmos do CircuitJS1.

## 2. Um circuito do zero

Um circuito RC em série, para conhecer o básico:

1. **Arquivo > Novo Circuito em Branco**.
2. Abra o menu **Desenhar** e escolha a fonte de tensão; ou use a barra de ferramentas. Clique
   no canvas e arraste: o elemento é desenhado do ponto onde você clicou até onde você soltar.
3. Faça o mesmo com um resistor (tecla **r**) e um capacitor (tecla **c**), e com o terra
   (tecla **g**).
4. Ligue os componentes com fios: tecla **w**, clique num terminal e arraste até o outro. Dois
   terminais no mesmo ponto da grade estão ligados.
5. Aperte **Esc** (ou a tecla de espaço) para voltar ao modo de seleção.
6. Dê um **clique duplo** num componente para editar o valor (por exemplo, o resistor para
   1 kΩ).
7. Clique com o botão direito na fonte ou no capacitor e escolha **Visualizar na Bancada**: um
   osciloscópio aparece embaixo.
8. Aperte **Executar** (no painel à direita). Para alterar a velocidade, use o controle
   **Velocidade da Simulação**.

Os passos abaixo detalham cada uma dessas ações.

## 3. A janela

```
┌───────────────────────────────────────────────────────────────┐
│ Menu: Arquivo Editar Desenhar Diagramas de Tempo Opções …     │
├───────────────────────────────────────────────────────────────┤
│ Barra de ferramentas (desfazer, zoom, componentes mais usados)│
├────────────────────────────────────────────┬──────────────────┤
│                                            │ Reiniciar        │
│            Canvas do circuito              │ Executar / Parar │
│                                            │ Velocidades      │
│                                            │ Brilho           │
├────────────────────────────────────────────┤ Circuito atual   │
│ Osciloscópios        │ Informações         │ Sliders          │
└────────────────────────────────────────────┴──────────────────┘
```

- **Canvas.** Onde o circuito é desenhado e editado. Usa a GPU quando há OpenGL. Se só houver
  renderização por software, o programa mostra um aviso e continua funcionando, mais devagar.
- **Painel à direita.** Largura inicial de 350 pixels; arraste o divisor para mudar. Contém:
  - **Reiniciar:** volta o tempo a zero e o circuito ao estado inicial.
  - **Executar / Parar:** liga e desliga o tempo.
  - **Velocidade da Simulação:** quantos passos são calculados por segundo.
  - **Velocidade da Corrente:** a velocidade dos pontos que mostram a corrente (não muda a
    física).
  - **Brilho da Potência:** o quanto a cor mostra a potência (ativo com **Mostrar Potência**).
  - **Circuito Atual:** o nome e os sliders do circuito (seção 7).
  - Um botão **▶ Tocar Áudio** para cada elemento de saída de áudio (seção 12).
- **Barra de título.** Mostra o tempo simulado (`t = …`).
- **Modo.** No canto direito da barra de ferramentas, o modo atual (por exemplo, *Modo Edição*).
- **Faixa de aviso.** Mensagens e erros aparecem numa faixa acima do canvas, com um botão
  **Fechar**.

## 4. Montar o circuito

### Colocar componentes

Há três caminhos:

1. **Menu Desenhar.** Lista todos os componentes por categoria (seção 11). Escolher um deixa o
   programa no modo de colocação.
2. **Barra de ferramentas.** Os componentes mais usados, com uma seta ao lado de alguns para
   abrir mais opções.
3. **Teclas.** Quase todos os componentes comuns têm uma letra (seção 13).

No modo de colocação, **clique e arraste**: o primeiro ponto é onde você clicou, o segundo é onde
você soltou, e ambos caem na grade. O modo continua ativo para você colocar mais de um; **Esc**
(ou o botão direito do mouse) sai dele.

### Fios

Escolha o fio (tecla **w**) e arraste de um ponto a outro. Um fio **roteado** (tecla
**Shift+w**) guarda os pontos do caminho; use-o quando quiser um traçado em L.

Terminais que caem no mesmo ponto estão ligados; um ponto branco marca uma junção de três ou
mais. O terra é um elemento: **toda parte do circuito precisa estar ligada a um terra** (ou a um
fonte de tensão que o defina), senão aparece o erro *Need to ground...* ou a matriz fica
singular.

### Selecionar

- Clique num componente para selecioná-lo; **Ctrl+clique** soma ou tira da seleção.
- Arraste num espaço vazio para desenhar um retângulo de seleção.
- **Editar > Selecionar Tudo** (Ctrl+A).
- Os componentes selecionados ficam em ciano.

### Mover e esticar

- **Arrastar um componente selecionado** o move (com tudo o que está selecionado junto).
- **Arrastar um terminal** de um componente isolado move só aquela ponta, esticando-o.
- O submenu **Desenhar > Drag** escolhe outros modos de arrasto:

| Modo | O que arrasta |
|------|---------------|
| Drag All | O circuito inteiro |
| Drag Row | Tudo o que está na linha onde você clicou |
| Drag Column | Tudo o que está na coluna onde você clicou |
| Drag Selected | Só a seleção |
| Drag Post | Todos os terminais que estão no ponto clicado |
| Select/Drag Sel (espaço) | Volta ao modo normal |

### Editar valores

- **Clique duplo** num componente abre uma janela com os campos dele: resistência, tensão,
  frequência, forma de onda, modelo, rótulos, e assim por diante. Os valores aceitam sufixos
  (`4k7`, `1u`, `2.2n`, `10meg`, `5 rms`).
- Alguns campos têm botões extras, como **Editar Modelo** (diodos, transistores, MOSFETs,
  relés) ou os de arquivo (seção 12).
- Com **Opções > Editar Valores com Roda do Mouse**, girar a roda sobre um componente muda o
  primeiro valor numérico dele em passos de 10%.

### Menu do botão direito

Clique com o botão direito num componente:

| Item | O que faz |
|------|-----------|
| Visualizar na Bancada | Cria um osciloscópio para o componente |
| Inserir Controles Deslizantes | Põe campos do componente no painel como sliders |
| Duplicar | Faz uma cópia ao lado |
| Excluir | Remove o componente |

Num osciloscópio solto no circuito, o menu é o do osciloscópio (seção 6).

### Editar o circuito

Menu **Editar**:

| Item | Atalho |
|------|--------|
| Desfazer / Refazer | Ctrl+Z / Ctrl+Y |
| Recortar / Copiar / Colar | Ctrl+X / Ctrl+C / Ctrl+V |
| Duplicar | Ctrl+D |
| Excluir | Delete |
| Selecionar Tudo | Ctrl+A |
| Procurar Componente… | `/` |
| Centralizar Circuito | |
| Zoom 100% / Mais / Menos | `0` / `+` / `-` |
| Espelhar X / Espelhar Y | |
| Girar (anti-horário / horário) | |

- **Procurar Componente** abre uma lista com campo de busca; escolher um item seleciona o
  componente e leva o canvas até ele.
- **Espelhar** e **Girar** valem para a seleção (ou para o circuito todo se nada estiver
  selecionado) e dá para desfazer.
- **Copiar** põe o texto do circuito na área de transferência, então dá para colar em outro
  programa, ou numa outra janela do prometheuscs.

### Zoom e movimento do canvas

- **Roda do mouse:** aproxima e afasta, em torno do ponteiro.
- **Shift + arrastar**, **botão do meio** ou **segurar a tecla de espaço e arrastar:** move o
  canvas.
- **Centralizar Circuito** ajusta o enquadramento ao circuito inteiro.
- A grade é de 16 pixels; **Opções > Grade Pequena** a reduz para 8.

## 5. Executar a simulação

- **Executar / Parar** liga e desliga o tempo; com ele parado o circuito pode ser editado e
  continua onde estava.
- **Reiniciar** volta o tempo a zero e restaura o estado dos componentes (a carga dos
  capacitores, a corrente dos indutores, o estado dos flip-flops).
- **Velocidade da Simulação:** sobe até muitos milhares de passos por segundo. O tamanho do passo
  de tempo vem do circuito (por padrão, 5 µs) e pode ser mudado em
  **Opções > Outras Opções**.
- **Cores e pontos.** Cada fio tem a cor da sua tensão (verde positivo, vermelho negativo,
  cinza zero); os pontos amarelos andam no sentido da corrente. Os controles estão em
  **Opções** (seção 10).
- **Interruptores.** Clique num interruptor para fechá-lo ou abri-lo durante a simulação. Alguns
  têm uma tecla própria, que se define no campo *Tecla de atalho* do interruptor; apertá-la
  também o aciona.
- **Encontrar Ponto CC** (menu Arquivo): calcula o ponto de operação em corrente contínua
  (capacitores abertos) e continua a simulação a partir dele. **Outras Opções > Encontrar o
  ponto CC ao reiniciar** faz isso a cada reinício.
- **Parar sozinho.** Alguns componentes interrompem o tempo: o *Stop Trigger* e erros do
  circuito. O motivo aparece na faixa de aviso e o componente responsável fica destacado.

### Interpretar os erros

Mensagens comuns, todas iguais às do CircuitJS1:

| Mensagem | Significado |
|----------|-------------|
| Singular matrix! | O circuito não tem solução única: um pedaço não está ligado ao terra, ou há uma fonte de tensão em curto |
| Convergence failed! | Os elementos não lineares não chegaram a um valor estável; tente um passo de tempo menor |
| Need to ground… | Falta uma referência de terra (linhas de transmissão, por exemplo) |
| Path to ground with no resistance! | Uma fonte ligada ao terra só por fios |
| capacitor exceeded max reverse voltage | Um capacitor polarizado foi ligado ao contrário |
| max current exceeded | Um diodo, LED ou transistor passou da corrente máxima |

## 6. Osciloscópios

Os osciloscópios mostram um valor ao longo do tempo. Ficam na faixa embaixo do canvas, ou
soltos dentro do circuito.

### Criar e remover

- **Visualizar na Bancada** (botão direito no componente, ou menu **Diagramas de Tempo**)
  cria um osciloscópio com a tensão e a corrente do componente.
- **Diagramas de Tempo > Remover Todos os Diagramas** apaga todos.
- No menu do osciloscópio: **Remover Osciloscópio**.

### Brilho do osciloscópio do componente selecionado

Quando você seleciona um componente (ou apenas passa o ponteiro sobre ele) que um osciloscópio
está plotando, o fundo desse osciloscópio ganha um brilho azulado suave, para achar o gráfico do
componente sem ler os rótulos. Vale para os osciloscópios da faixa e para os soltos no circuito,
em qualquer tamanho. O brilho some ao desselecionar.

### Menu do osciloscópio

Clique com o botão direito num osciloscópio:

- **Valores a plotar:** tensão, corrente, potência e outros valores do componente (carga, e as
  correntes e tensões dos transistores, por exemplo), marcando ou desmarcando.
- **Escala Máxima:** a escala só cresce.
- **Mostrar V vs I**, **Plot X/Y**, **Mostrar Espectro:** modos de desenho (abaixo).
- **Empilhar / Desempilhar / Combinar:** organização (abaixo).
- **Remover Plot:** tira um dos valores (quando há vários).
- **Reiniciar:** apaga o histórico.
- **Exportar CSV…:** grava as amostras (tempo e mínimo e máximo de cada valor).
- **Propriedades…:** a janela completa (abaixo). Também abre com a engrenagem no canto inferior
  esquerdo de cada osciloscópio, ou com clique duplo num osciloscópio solto.
- **Desacoplar / Acoplar:** tira o osciloscópio da faixa e o põe como um retângulo no circuito, e
  o contrário.
- **Roda do mouse** sobre um osciloscópio: muda a escala de tempo (rápido ou lento).

### Organização dos osciloscópios

Cada osciloscópio ocupa uma coluna; as colunas ficam lado a lado.

- **Empilhar:** põe um osciloscópio na coluna do anterior (os dois ficam um sobre o outro, cada
  um com o seu retângulo).
- **Desempilhar:** volta a dar uma coluna ao osciloscópio.
- **Combinar:** passa os valores de um osciloscópio para o anterior, que passa a desenhar todos
  juntos.
- No menu **Diagramas de Tempo**, as versões para todos de uma vez: **Empilhar Todos,
  Desempilhar Todos, Combinar Todos, Separar Todos**.

Quando a área é larga, uma coluna de informação aparece à direita: o tempo (`t = …`), o passo de
tempo e, sob o ponteiro, as informações do componente tocado (ou do que o osciloscópio mede).

### O que se vê num osciloscópio

- A **grade** e a escala: no modo automático a escala acompanha o sinal em passos de 1-2-5; se o
  sinal nunca é negativo, o zero fica embaixo.
- As **curvas**, cada uma na sua cor (verde para tensão, amarelo para corrente). A curva mais
  perto do ponteiro fica mais grossa.
- O **tempo por divisão** no canto inferior direito. Esse texto, os valores da escala nos cantos
  da esquerda e as medidas (como `Max=`) também ficam sobre placas translúcidas.
- Ao passar o mouse, uma linha vertical com os valores e o tempo naquele ponto. Cada rótulo
  fica sobre uma placa escura e translúcida (branca no fundo claro), para ler bem por cima da
  curva e da grade. O nome do componente e os valores atuais aparecem na área de informações, à
  direita dos osciloscópios.
- **Arrastar** sobre o gráfico mede um trecho: aparecem `Δt` e `Δ` (a variação do valor), também
  sobre placas.
- Passar o mouse sobre um osciloscópio destaca em ciano os componentes que ele mede, com `+` e
  `−` nos terminais.

Com o botão pressionado sobre uma curva no **modo de escala manual**, o arrasto vertical move a
curva.

### Propriedades

A janela de propriedades tem:

- **Escala manual** e **escala máxima**, e o número de divisões.
- **Passos de tempo por amostra** (a escala de tempo).
- **Gatilho (trigger):** modo (livre, normal, automático), borda (subida ou descida) e nível. O
  gatilho congela a imagem no ponto em que o sinal cruza o nível.
- Para cada valor: **escala por divisão** e **posição vertical**, e **acoplamento CA** (tira a
  componente contínua de uma tensão).
- **Medidas:** valor de pico, pico negativo, pico a pico, frequência, valor RMS, média, ciclo de
  trabalho, escala, informação estendida do componente e ângulo de fase.
- **Espectro em escala logarítmica** (dB).
- **Eixos do X/Y:** quais valores vão em X, em Y, no brilho e nas cores vermelha, verde e azul, e
  a persistência do rastro.

### Modos de desenho

- **Tempo** (padrão): valores contra o tempo.
- **Mostrar V vs I:** tensão (horizontal) contra corrente (vertical). Serve para ver o
  comportamento de um componente, como o laço de um capacitor ou a curva de um diodo. O eixo
  vertical é amarelo.
- **Plot X/Y:** qualquer valor contra qualquer outro; precisa de dois valores no osciloscópio.
  Os eixos são verdes. É o modo das figuras de Lissajous. Os valores de brilho e cor deixam um
  valor controlar o brilho ou a cor do traço, o que permite desenhar imagens (veja os exemplos do
  grupo *2-D Scope*: o tabuleiro, o tabuleiro colorido e o rosto sorridente). O traço fica na
  tela e esmaece devagar, como num osciloscópio de fósforo.
- **Mostrar Espectro:** a transformada de Fourier do primeiro valor, com a frequência do maior
  pico e, ao passar o mouse, a frequência e o nível em dB. Com *Espectro (logarítmico)*, a
  escala é em dB (de 0 a −80).

### Osciloscópio solto no circuito

**Desacoplar** transforma o osciloscópio num retângulo dentro do circuito (como o *Scope* do
original). Ele:

- se **move** arrastando por qualquer ponto;
- se **redimensiona** arrastando um dos cantos, que ficam marcados quando ele está selecionado;
- mostra a escala, as medidas e as leituras só quando está grande o bastante (a partir de 160 por
  90 pixels); pequeno, só desenha as curvas;
- tem engrenagem, menu e propriedades como os outros;
- é salvo junto com o circuito.

## 7. Sliders

Os sliders deixam você mudar um valor com o circuito rodando.

- **Botão direito num componente > Inserir Controles Deslizantes.** A janela lista os campos
  numéricos do componente. Para cada um que você ligar, defina o mínimo, o máximo, o passo, se a
  escala é linear ou logarítmica e o texto que aparece no painel.
- Os sliders aparecem no painel à direita, sob **Circuito Atual**, com o valor ao lado.
- Para mudar ou tirar um slider, abra de novo **Inserir Controles Deslizantes** no mesmo
  componente.
- Os sliders são salvos com o circuito. Os exemplos do CircuitJS1 já trazem os deles (por
  exemplo, capacitância, indutância e resistência no *Circuito RLC*).

## 8. Arquivos, exemplos e compartilhamento

Menu **Arquivo**:

| Item | O que faz |
|------|-----------|
| Nova Janela… (Ctrl+N) | Abre outra janela do programa |
| Novo Circuito em Branco | Esvazia o circuito |
| Abrir Arquivo… (Ctrl+O) | Abre um circuito (`.txt`, `.xml` ou `.circuitjs`) |
| Importar de Texto… | Cola o texto de um circuito |
| Salvar (Ctrl+S) / Salvar Como… (Ctrl+Shift+S) | Grava o circuito |
| Exportar Como Link… | Mostra um link de compartilhamento |
| Exportar Como Texto… | Mostra o texto do circuito |
| Salvar Como Imagem… | Grava um PNG |
| Copiar Imagem do Circuito para a Área de Transferência | |
| Salvar Como SVG… | Grava um desenho vetorial |
| Criar Subcircuito… | Veja a seção 9 |
| Encontrar Ponto CC | Seção 5 |
| Recuperar Salvamento Automático | Reabre a cópia automática |
| Imprimir… (Ctrl+P) | Imprime ajustado à página |
| Tela Cheia (F11) | |
| Sobre… | |

- **Formatos.** O programa abre e grava os dois formatos do CircuitJS1: o **texto** (uma linha
  por componente, que começa com `$` e as opções do circuito) e o **XML** (`<cir>…</cir>`). Ele
  grava XML. Circuitos feitos no CircuitJS1 abrem aqui, e os daqui abrem lá.
- **Links.** *Exportar Como Link* gera o mesmo tipo de link que o CircuitJS1 (com o circuito
  comprimido no parâmetro `ctz`), no formato do CircuitJS1. A janela *Importar de Texto* só aceita o texto do circuito, não um link.
- **Arquivos com problemas.** Se uma linha de um arquivo não puder ser lida (componente
  desconhecido, campos faltando), o resto do circuito carrega, e uma janela lista cada linha,
  o tipo e o motivo.
- **Salvamento automático.** A cada 30 segundos, se há alterações, o programa guarda uma cópia
  (pasta de configuração, seção 15). Se o programa fechar sem salvar, use **Recuperar
  Salvamento Automático**.
- **Exemplos.** O menu **Exemplos de Circuitos** reúne os 373 circuitos em grupos. Abrir um
  exemplo substitui o circuito atual.
- **Imagem e impressão** usam o mesmo desenho do canvas, inclusive com **Fundo Branco**
  (seção 10), o que gasta menos tinta.

## 9. Subcircuitos

Um subcircuito vira um componente (um "chip") a partir de um pedaço do circuito.

**Criar.**

1. Monte o pedaço do circuito. Os pontos de entrada e saída são **Labeled Node** (nós
   rotulados, no menu Desenhar > Outputs and Labels). Cada rótulo vira um pino com o mesmo nome,
   exceto os marcados como internos.
2. Selecione o que vai entrar no chip (ou nada, para usar o circuito todo).
3. **Arquivo > Criar Subcircuito…**
4. Na janela, dê um nome, escolha o tamanho do chip, o lado e a posição de cada pino, se o nome
   aparece no chip, e onde ele fica guardado. A janela mostra o chip: arraste um pino para
   mudá-lo de lugar (ele vai para o lugar livre mais próximo), ou use as linhas abaixo da imagem:
   - **neste circuito** (vai junto no arquivo);
   - **nesta sessão** (some quando o programa fecha);
   - **salvo** (fica disponível em todos os circuitos).

O novo subcircuito já fica escolhido para ser colocado.

**Usar.** Cada subcircuito criado aparece em **Desenhar > Subcircuitos**, ou no **Gerenciador de Subcircuitos**
(**Ferramentas > Subcircuit Manager**), que lista os existentes com botões para colocar, editar o
layout dos pinos e apagar.

**Editar uma instância.** Clique duas vezes numa instância:

- **Editar Layout de Pinos:** a mesma janela da criação.
- **Ver componentes:** mostra o que há dentro, com as tensões e correntes da simulação em
  andamento, só para leitura. **Voltar** na barra sai.
- **Editar Modelo:** deixa o seu circuito de lado e abre o circuito do subcircuito. Uma barra
  mostra *Editando: nome*, com **Voltar** (descarta), **Salvar** (grava, mantém os pinos e o
  lugar onde o modelo estava guardado e volta ao seu circuito) e **Salvar Cópia** (grava com
  outro nome).
- Também dá para mudar a *tensão lógica alta* da instância.

Subcircuitos podem conter outros subcircuitos (até 16 níveis).

## 10. Opções

Menu **Opções**. Os itens marcados ficam gravados e valem na próxima vez que você abrir o
programa.

| Opção | Efeito |
|-------|--------|
| Mostrar Corrente | Pontos amarelos andando nos fios |
| Mostrar Tensão | Fios coloridos pela tensão |
| Mostrar Potência | Componentes coloridos pela potência (use o controle *Brilho da Potência*) |
| Mostrar Valores | Escreve o valor ao lado de cada componente |
| Grade Pequena | Grade de 8 em vez de 16 pixels |
| Barra de Ferramentas | Mostra ou esconde a barra |
| Mostrar Cursor em Cruz | Linhas guia que passam pelo ponteiro |
| Resistores Europeus | Desenha o resistor como retângulo |
| Mostrar Unidade Ω | Escreve o símbolo Ω junto ao valor |
| Portas IEC | Portas lógicas em retângulos normalizados |
| Fundo Branco | Para impressão |
| Sentido Convencional de Corrente | Os pontos andam do positivo para o negativo |
| Desativar Edição | Trava o circuito: só dá para executar e olhar |
| Editar Valores com Roda do Mouse | Seção 4 |
| Idioma | 16 línguas; vale na próxima abertura do programa |
| Atalhos… | Lista os atalhos de teclado |
| Outras Opções… | Veja abaixo |

**Outras Opções.**

- **Passo de tempo:** o maior passo (em segundos). Passos menores dão mais precisão e menos
  velocidade.
- **Faixa de tensão para cores:** a tensão que corresponde ao verde (ou vermelho) mais forte.
- **Ajustar o passo de tempo automaticamente:** diminui o passo quando a convergência falha.
- **Encontrar o ponto CC ao reiniciar.**

## 11. Componentes

O menu **Desenhar** lista por categoria tudo o que existe (todos os componentes do CircuitJS1).
As teclas de atalho estão na seção 13.

| Categoria | Componentes |
|-----------|-------------|
| (sem categoria) | Fio, fio roteado, resistor |
| Inputs and Sources | Terra, fonte CC, fonte CA, trilhos (CC, CA, quadrado, variável), gerador de clock, fonte de corrente, bateria, varredura de frequência, AM, FM, entrada de dados, entrada de áudio |
| Passive Components | Potenciômetro, capacitor (inclusive polarizado), indutor, transformadores (comum, com derivação, personalizado), linha de transmissão, interruptores (simples, de pressão, duplo, de dois polos, cruzado, make-before-break), centelhador, memristor, lâmpada, fusível, LDR, termistor NTC, cristal |
| Outputs and Labels | Medidores (voltímetro, ohmímetro, amperímetro, wattímetro, ponta de prova), saída, texto, caixa, linha, nó rotulado, gatilho de parada, gravador de dados, ponto de teste, motores (CC, trifásico, chave de proteção), LED, saída de áudio |
| Switches | Relé e as partes de bobina e contato |
| Logic Gates, Input and Output | Entrada e saída lógica, inversor, NAND, NOR, AND, OR, XOR, XNOR, buffer com atraso, tri-state |
| Active Building Blocks | Schmitt (e inversor Schmitt), amp-op (e com entradas trocadas), fontes controladas (VCVS, VCCS, CCVS, CCCS), girador, chaves analógicas |
| Active Components | Diodos (comum, Zener, varactor, túnel), transistores (NPN, PNP, Darlington), triodo, SCR, diac, triac, MOSFETs e JFETs (canal N e P), transistor de unijunção, comparador, amplificador de Norton, OTA, optoacoplador, amp-op real (LM741, LM324) |
| Analog and Hybrid Chips | 555, comparador de fase, VCO, conveyor de corrente (CC2), relé com atraso, multiplexador analógico, ADC, DAC |
| Digital Chips | Flip-flops (D, JK, T), latch, contadores, anel, monoestável, somadores, multiplexador e demultiplexador, displays (sete segmentos, decimal, matriz de LED, instrução), SRAM e ROM, registradores de deslocamento, gerador de sequência, barramentos (separador, transceptor, entrada), lógica personalizada |
| Subcircuits | Instâncias de subcircuitos |

Cada componente tem um **clique duplo** com os seus campos, e **a janela de edição** explica o
que cada um faz. A **caixa de informações** (a coluna à direita dos osciloscópios) mostra o que o
componente sob o ponteiro está fazendo: tensão, corrente, potência e valores próprios.

### Componentes especiais

- **Nó rotulado (Labeled Node):** liga pontos distantes sem fio: dois nós com o mesmo rótulo estão
  ligados. É também o que define os pinos de um subcircuito.
- **Lógica personalizada (Custom Logic):** um chip cujo comportamento é escrito como regras no modelo do
  elemento, que se edita nos campos dele.
- **Texto, caixa e linha:** anotações; não participam da simulação.
- **Cor (LED e texto):** nas propriedades do LED e do texto a cor se escolhe com um seletor de cor
  (um botão com a cor atual), não digitando valores.
- **Gatilho de parada (Stop Trigger):** pára o tempo quando a entrada passa de um nível.
- **Gravador de dados (Data Export):** guarda o que passou pelo terminal; grava num arquivo
  (seção 12).
- **Linha de transmissão:** a cor do interior da linha mostra a tensão que viaja nela.
- **Lâmpada:** o brilho segue a temperatura do filamento.

## 12. Áudio, dados e arquivos de amostras

- **Saída de áudio.** Grava a tensão do terminal como som (a faixa de −1 a 1 V é a que vira
  som; fora dela corta). Há uma duração (até 5 s) e uma taxa de amostragem (8000 a 48000 Hz).
  No painel à direita aparecem **▶ Tocar Áudio** e um botão de salvar como `.wav`.
  - Toque depois de deixar o circuito rodar por um tempo: o botão toca o que já foi gravado.
  - O som sai pelo `pw-play`, pelo `paplay` ou pelo `aplay`, o primeiro que existir; sem
    nenhum deles, o botão avisa.
  - O terminal precisa estar num ponto com tensão: se estiver ligado ao terra, o que se grava é
    silêncio.
- **Entrada de áudio.** Uma fonte que toca um arquivo `.wav` (PCM de 8 a 32 bits, ou ponto
  flutuante de 32 e 64 bits; usa o primeiro canal), com tensão máxima e posição inicial.
- **Entrada de dados.** Lê números de um arquivo de texto e os usa como tensão ao longo do tempo
  (com fator de escala, duração de cada amostra e repetição).
- **Gravador de dados.** Grava o que recebeu num arquivo de texto.

Nos três, o campo do caminho do arquivo tem um **botão de abrir arquivo** ao lado; dá para
digitar o caminho também.

## 13. Atalhos de teclado e mouse

**Teclas** (também em **Opções > Atalhos…**):

| Tecla | Faz |
|-------|-----|
| Ctrl+N / O / S / Shift+S / P | Nova janela / Abrir / Salvar / Salvar como / Imprimir |
| Ctrl+Z / Y | Desfazer / Refazer |
| Ctrl+X / C / V / D | Recortar / Copiar / Colar / Duplicar |
| Delete | Excluir |
| Ctrl+A | Selecionar tudo |
| `/` | Procurar componente |
| `0`, `+`, `-` | Zoom 100%, mais, menos |
| F11 | Tela cheia |
| Esc | Volta ao modo de seleção |
| Espaço | Segurar e arrastar: mover o canvas |
| `w` | Fio |
| Shift+w | Fio roteado |
| `r` | Resistor |
| `c` | Capacitor |
| Shift+c | Capacitor polarizado |
| `l` | LED |
| Shift+l | Indutor |
| `d` | Diodo |
| `z` | Diodo Zener |
| `n` / `p` | Transistor NPN / PNP |
| Shift+n | MOSFET de canal N |
| `a` | Amp-op |
| Shift+a | Amp-op (entradas trocadas) |
| `g` | Terra |
| `i` / `o` | Entrada / saída lógica |
| `1` | Inversor |
| *(letra do interruptor)* | Aciona os interruptores com essa tecla |

**Mouse:**

| Gesto | Faz |
|-------|-----|
| Clique | Seleciona, ou aciona um interruptor |
| Clique duplo | Edita o componente |
| Clique + arrasto | Move (num componente selecionado), seleciona (no vazio), ou desenha (no modo de colocação) |
| Clique direito num componente | Menu do componente |
| Clique direito num osciloscópio | Menu do osciloscópio |
| Roda | Zoom, ou muda o valor (com a opção ligada), ou muda a escala de tempo (sobre um osciloscópio) |
| Shift + arrasto, botão do meio | Move o canvas |

## 14. Quando algo dá errado

- **O circuito não anda / uma faixa de aviso aparece.** Leia a mensagem (seção 5). O componente
  responsável fica destacado; confira o terra e as ligações.
- **Um arquivo abre com uma lista de avisos.** O programa ignorou as linhas que não conseguiu
  ler, e o resto do circuito carregou. Isso acontece, por exemplo, com componentes de versões
  mais novas do CircuitJS1 que ainda não existem aqui.
- **Os osciloscópios mostram linhas retas.** O circuito está parado, ou o valor mostrado é
  constante. Confira *Executar* e a escala.
- **Não sai som.** Veja a seção 12: o terminal tem de estar num nó com tensão, o circuito tem de
  ter rodado por um tempo, e um dos três tocadores precisa estar instalado.
- **Desenho lento.** Provavelmente não há aceleração gráfica; o programa avisa disso ao abrir.
  Para circuitos grandes, diminua a **Velocidade da Simulação**.
- **Fechei sem salvar.** **Arquivo > Recuperar Salvamento Automático**.
- **Não consigo editar.** Confira **Opções > Desativar Edição**, e se você não está olhando
  *dentro* de um subcircuito (modo *Ver componentes*, só leitura).

## 15. Linha de comando e arquivos do programa

**Linha de comando.**

```bash
prometheuscs                                   # abre o programa
prometheuscs --bench-render ARQUIVO [--bench-seconds N]   # mede o desenho e sai
```

`--bench-render` abre a janela com o circuito, mede quadros por segundo, tempo de desenho,
tempo até o primeiro quadro e latência de clique, imprime o resultado e fecha (veja
[tests/bench/README.md](../tests/bench/README.md)).

Com `RUST_LOG=prometheuscs=debug`, o programa imprime mensagens de diagnóstico.

**Arquivos.** Na pasta de configuração (`$XDG_CONFIG_HOME/prometheuscs`, ou
`~/.config/prometheuscs`):

| Arquivo | Conteúdo |
|---------|----------|
| `settings.json` | As opções marcadas e o idioma |
| `autosave.xml` | A cópia automática do circuito |
| `subcircuits/*.xml` | Os subcircuitos guardados como "salvos" |

## 16. Diferenças em relação ao CircuitJS1

O objetivo é que os circuitos se comportem como no original e que a interface seja familiar.
Diferenças conhecidas:

- É um programa de janela do GNOME (GTK4 / libadwaita), não uma página web: não há as funções
  que só fazem sentido no navegador (importar do Dropbox, o modo incorporado em páginas, a
  interface de script).
- O áudio toca por programas do sistema (`pw-play`, `paplay` ou `aplay`), não por uma biblioteca
  própria.
- As imagens do osciloscópio X/Y são feitas com 512 por 192 células e esticadas; a duração do
  rastro não é idêntica à do original.
- O osciloscópio solto, na janela de propriedades e na engrenagem, se comporta como o do
  original, mas ao mudar suas configurações o histórico dele recomeça.
- Resultados numéricos foram conferidos contra o CircuitJS1 com gravações de 373 circuitos; as
  diferenças que se conhecem estão em `tests/compat/known_differences.json`.
