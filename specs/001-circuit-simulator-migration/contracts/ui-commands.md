# Contract: UI Commands and Layout

Every command below is a named application action available from its menu, and, where listed, a keyboard shortcut. This is the interface contract for FR-002 to FR-006, FR-014, FR-015 and SC-007. Labels shown are the Portuguese defaults from the reference screenshots; English keys come from the original.

## Window layout

Top to bottom: menu bar; toolbar (hidable) with mode indicator ("Modo: Modo Edição"); main area with the circuit canvas on the left and the side panel on the right; scope panel along the bottom of the canvas area.

## Side panel

Restart, Stop buttons; sliders: simulation speed, current speed, brightness (disabled when not applicable); label "Circuito Atual:" with the circuit title; one slider per circuit adjustable (capacitance, inductance, resistance, and so on). Each slider has decrement/increment arrows.

## Menus

| Menu | Entries (shortcut) |
|------|--------------------|
| Arquivo | Nova Janela (Ctrl-N), Novo Circuito, Abrir Arquivo (Ctrl-O), Importar de Arquivo Texto, Salvar (Ctrl-S), Salvar como, Exportar como Link, Exportar em Formato Texto, Exportar como Imagem, Copiar imagem para a área de transferência, Exportar como SVG, Criar Subcircuito, Encontrar Ponto de Operação CC, Recuperar Salvamento Automático, Imprimir (Ctrl-P), Alternar Tela Cheia, Sobre |
| Editar | Desfazer (Ctrl-Z), Refazer (Ctrl-Y), Recortar (Ctrl-X), Copiar (Ctrl-C), Colar (Ctrl-V), Duplicar (Ctrl-D), Selecionar Tudo (Ctrl-A), Encontrar Componente (/), Centralizar o Circuito, Zoom a 100% (0), Mais Zoom (+), Menos Zoom (-), Espelhar X, Espelhar Y, Rotacionar Anti-Horário, Rotacionar Horário |
| Desenhar | Inserir Conexão Elétrica (w), Adicionar cabo enrutado (W), Inserir Resistor (r), submenus: Componentes Passivos, Entradas e Fontes, Saídas e Rótulos, Componentes Ativos, Blocos de Construção Ativos, Portas Lógicas Entrada e Saída, CIs Digitais, CIs Analógicos e Híbridos, Subcircuitos, Arrastar; toggle Modo Edição (Espaço ou Shift+arrastar) |
| Diagramas de Tempo | Empilhar Todos, Desempilhar Todos, Combinar Todos, Separar Todos |
| Opções | Toggles: Mostrar Corrente, Mostrar Tensão, Mostrar Potência, Mostrar Valores, Graticula Pequena, Barra de ferramentas, Mostrar Cursor em Cruz, Resistores c/ Símbolo Europeu, Show Ω Unit, Porta Padrão IEC, Fundo de Tela Branco, Sentido Convencional de Corrente, Desativar Edição, Editar Valores com Roda do Mouse; dialogs: Desvios, Outras Opções |
| Ferramentas | Converter fios em fios enrutados, Gerenciador de subcircuitos |
| Exemplos de Circuitos | Submenus generated from the example catalog (`setuplist.txt`): Básicos, Circuitos CA, Filtros Passivos, and so on, plus Novo Circuito |

The "Ferramentas de Desenvolvimento do Navegador" entry from the reference is intentionally omitted (browser-only, see spec assumptions).

## Toolbar

Undo, Redo, Cut, Copy, Paste, Duplicate, Find, Zoom 100%, Zoom In, Zoom Out, then component quick-insert buttons (wire, resistor, ground, capacitor, inductor, and the other quick-insert parts of the original toolbar) that also support press-and-drag placement. Icons follow research R-7.

## Behavior contracts

- **C1** Toggling an Options item updates the canvas within one frame and persists the setting.
- **C2** Every Draw-menu entry creates the matching element type in placement mode; the set of entries equals the registry contents and therefore the component inventory (SC-010, FR-025).
- **C3** Disabled states: Undo/Redo, Cut/Copy, Save are disabled when not applicable, as shown in the screenshots.
- **C4** Keyboard shortcuts match the table above and are listed in an in-app shortcuts dialog.
- **C5** Simulation errors appear as a dismissible banner naming the element; the offending element is highlighted on the canvas.
- **C6** Application actions are testable without a display through the app crate's action layer (`dispatch(Action) -> State`), with GTK glue kept thin.
