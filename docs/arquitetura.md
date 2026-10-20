# Arquitetura

Este documento é para quem vai manter ou estender o prometheuscs. Ele explica como o programa
está dividido, por onde os dados andam, e onde mexer para cada tipo de mudança. Para usar o
programa, veja o [guia do usuário](guia-do-usuario.md); para criar um componente, veja
[adding-a-component.md](adding-a-component.md).

Os números de linhas e arquivos citados são de quando o documento foi escrito; os nomes de
módulos e de funções são o que importa.

## Sumário

1. [Visão geral](#1-visão-geral)
2. [Mapa do código](#2-mapa-do-código)
3. [O motor](#3-o-motor)
4. [Componentes](#4-componentes)
5. [Osciloscópios](#5-osciloscópios)
6. [A aplicação](#6-a-aplicação)
7. [Desenho](#7-desenho)
8. [Como as peças trabalham juntas: três percursos](#8-como-as-peças-trabalham-juntas-três-percursos)
9. [Famílias de componentes (features)](#9-famílias-de-componentes-features)
10. [Testes e verificação](#10-testes-e-verificação)
11. [Convenções do código](#11-convenções-do-código)
12. [Armadilhas conhecidas](#12-armadilhas-conhecidas)
13. [Receitas: onde mexer para…](#13-receitas-onde-mexer-para)

---

## 1. Visão geral

O prometheuscs é um **porte** do CircuitJS1 (Java/GWT) para Rust. A prioridade do projeto é o
**comportamento idêntico ao original** (os mesmos circuitos dão os mesmos resultados, os mesmos
arquivos abrem), e por isso boa parte do código é uma tradução fiel, com nomes e estruturas que
se reconhecem ao lado do Java. Quando você tiver dúvida de como algo *deve* se comportar, o
original em `/home/jgardona/Workspace/Rust/circuitjs1` é a referência.

Três ideias organizam tudo:

1. **O motor não conhece a interface.** Formato de arquivo, componentes, solver, osciloscópios e
   expressões compilam e rodam sem GTK. Tudo o que é de janela está em `src/app` (parte dele sob o
   feature `gui`) e em `src/render`.
2. **A simulação roda numa thread própria**, e a janela só recebe cópias do estado (os
   *snapshots*). A janela nunca espera o solver.
3. **Os componentes são dados descritos num registro.** Menus, fábrica, atalhos e a lista de
   componentes saem do registro; nenhum outro código conhece uma classe de componente pelo nome
   (com poucas exceções documentadas na seção 12).

```
                 ┌─────────────────────────── app ────────────────────────────┐
  arquivo ──►    │ Session (documento) ◄── snapshots ◄── Worker (thread)     │
  texto/XML      │   │  edição, desfazer, sliders          │ Session própria  │
  parser         │   ▼                                     ▼                  │
     │           │ window.rs (GTK) ── desenha ──► render (Painter: GL/Cairo/SVG)
     ▼           └────────────────────────────────────────────────────────────┘
  Circuit ─► Simulation ─► Elements (Box<dyn Element>) ─► Stamper ─► matriz ─► LU
  (format)    (solver)       (elements)                     (solver)
                 │
                 └──► Scope (scope): amostras, gatilho, FFT, imagem X/Y
```

## 2. Mapa do código

O pacote é único (`prometheuscs`), com uma biblioteca e alguns binários.

| Caminho | Papel |
|---------|-------|
| `src/lib.rs` | Raiz da biblioteca; a documentação do crate e um exemplo executável |
| `src/format/` | Texto e XML dos circuitos: `tokenizer`, `parser`, `writer`, `xml`, `circuit` (o documento lido), `number`, `link` (links `ctz`) |
| `src/elements/` | O trait `Element`, o registro, e **todos** os componentes (um arquivo por grupo) |
| `src/solver/` | `sim` (análise e passo), `stamper` (matrizes), `sparse_lu`, `dense_lu`, `path` (busca de laços), `snapshot` |
| `src/scope/` | Modelo do osciloscópio (`mod`), transformada de Fourier (`fft`), imagem do X/Y (`trace`) |
| `src/expr/` | O avaliador de expressões usado pela lógica personalizada e por outros componentes |
| `src/render/` | O trait `Painter` e seus backends (`gl`, `cairo`, `svg`), a cena, os osciloscópios, a exportação |
| `src/app/` | A aplicação: sessão, trabalhador, comandos, janela, diálogos |
| `src/error.rs`, `src/locale.rs` | Erros tipados; traduções |
| `src/bin/` | `import_assets`, `inventory`, `golden`, `bench_circuits`, `gl_spike` |
| `assets/` | Os 373 exemplos, a lista de exemplos, traduções, ícones e `inventory.json` |
| `tests/`, `benches/` | Testes de integração e comparação com o original; benchmarks |
| `specs/001-…/` | Especificação, plano, pesquisa e a lista de tarefas da migração |

Dependências do módulo (uma seta é "usa"):

```
app ──► render ──► elements ──► solver ──► format
 │         │           │            │
 │         └──► scope ◄┘            └──► error
 └──► (tudo)                expr ◄── elements
```

`format`, `elements`, `solver`, `scope` e `expr` não usam `app` nem `render`. Se você achar que
precisa, o código provavelmente deveria ficar do outro lado. (Há uma exceção histórica: alguns
componentes chamam `render::painter` para se desenhar, porque o `Painter` é a abstração do
desenho e não depende de nenhum backend.)

## 3. O motor

### 3.1 Do arquivo à simulação

1. `format::parser::parse(text, &registry)` lê o texto (ou o XML, se começa com `<`) e devolve um
   `ParseResult`: o `Circuit` (opções, componentes, osciloscópios, sliders, modelos) e a lista
   de erros de carga. **Uma linha ruim não impede o resto**: ela vira um `LoadError` e a
   leitura continua.
2. `Simulation::new(elements, settings)` guarda os componentes.
3. `Simulation::analyze()` (em `solver/sim.rs`) prepara tudo:
   - `calculate_wire_closure`: une pontos ligados por fios (uma estrutura de classes de
     equivalência, `PointClasses`);
   - `set_ground_node`, `make_node_list`: numera os nós, com o nó 0 como terra;
   - `calc_wire_info`, `find_unconnected_nodes`, `calculate_closures`: informação de fios e de
     quais nós se ligam por dentro de cada componente;
   - `pre_stamp_circuit` e `assign_matrix`: pede a cada componente o número de nós internos e de
     fontes de tensão, e separa o circuito em **uma matriz por grupo de nós ligados**;
   - `stamp_circuit`: cada componente "carimba" sua parte linear nas matrizes; as matrizes que
     são só lineares são fatoradas uma vez.
4. `step()` avança o tempo.

### 3.2 O passo de tempo (`Simulation::step`)

É a tradução do laço do original:

```
repita:
    se houve 3 iterações boas e o passo está reduzido: dobra o passo e carimba de novo
    para cada componente: start_iteration()
    para subiter em 0..limite (100 se ajusta o passo, senão 5000):
        converged = true
        restaura as matrizes das cópias lineares
        para cada componente: do_step()          ← componentes não lineares carimbam aqui
        para cada matriz: se não linear, fatora; resolve; aplica a solução
        se o circuito é linear, ou convergiu depois da 1ª passada: sai
    se não convergiu: reduz o passo (se permitido) ou "Convergence failed!"
    t += passo
    para cada componente: step_finished()
    para cada osciloscópio: time_step()          ← grava as amostras
    calcula as correntes dos fios (a menos que o cálculo seja adiado)
```

Erros viram `StepError` (`Singular`, `NotConverged`, `NotFinite`, `Stopped`) e a mensagem fica
em `Stamper::stop`. **Nunca há pânico por causa de um circuito ruim**: é um dos critérios do
projeto (SC-009), e `tests/edge_cases.rs` o testa com milhões de entradas geradas.

### 3.3 O `Stamper`

`solver/stamper.rs` guarda os nós, as fontes de tensão, as matrizes (`CircuitMatrix`: a matriz
original, a de trabalho, o lado direito, as tensões) e o estado do passo (tempo, passo, iteração,
`converged`, pedido de parada). Os componentes só falam com ele pelas funções `stamp_*`:

| Função | O que carimba |
|--------|---------------|
| `stamp_resistor`, `stamp_conductance` | Uma condutância entre dois nós |
| `stamp_voltage_source`, `_var`, `_saved` | Uma fonte de tensão (ideal, de valor mutável, ou só o lado direito) |
| `stamp_current_source` | Uma corrente entre dois nós (lado direito) |
| `stamp_vcvs`, `stamp_vccs`, `stamp_cccs` | Fontes controladas |
| `stamp_matrix`, `stamp_right_side` | Qualquer entrada, para os casos especiais |

Um componente linear carimba tudo em `stamp()` (uma vez); um não linear carimba sua parte
linear em `stamp()` e a parte que muda a cada iteração em `do_step()`.

### 3.4 O solver linear

`sparse_lu.rs` (esparsa, estilo CSparse, pivotamento parcial) e `dense_lu.rs`. A escolha é
automática (`SolverType::Auto`): matrizes com 150 linhas ou mais usam a esparsa
(`SPARSE_THRESHOLD`). A estrutura simbólica não é reaproveitada entre fatorações (o T122, de
otimização, foi fechado por não ser necessário: os benchmarks mostram folga).

### 3.5 Busca de laços (`solver/path.rs`)

Alguns componentes se validam procurando laços (uma fonte de tensão em curto, um indutor sem
caminho de corrente). `Validator::find_path(PathKind, …)` faz uma busca em profundidade. Serve
também a `validate_rail_node` e aos testes de "Path to ground with no resistance!".

### 3.6 O relógio e a análise CC

`sim_clock()` e `dc_analysis()` são variáveis da thread (`thread_local`) que os componentes lêem:
o tempo atual, o passo, e se estamos no cálculo do ponto de operação CC. É uma herança do original
(onde eram campos globais), mantida para não mudar a assinatura de centenas de métodos.
`find_dc_operating_point` liga o modo CC, analisa, dá um passo, volta e analisa de novo.

## 4. Componentes

### 4.1 O trait `Element` (`elements/element.rs`)

Uma tradução de `CircuitElm`. Quase todos os métodos têm implementação padrão, de modo que um
componente novo só define o que o diferencia. Os grupos:

| Grupo | Métodos principais |
|-------|--------------------|
| Identidade e arquivo | `class_name`, `dump_type`, `xml_dump_type`, `dump`, `dump_xml`, `undump_xml`, `dump_model_xml`, `dump_xml_state` |
| Geometria | `post_count`, `get_post`, `set_points`, `num_handles`, `handle_point`, `drag`, `flip_*`, `mouse_distance`, `click_rect` |
| Eletricidade | `stamp`, `do_step`, `start_iteration`, `step_finished`, `reset`, `non_linear`, `voltage_source_count`, `internal_node_count`, `get_connection`, `get_matrix_connection`, `has_ground_connection`, `validate` |
| Resultados | `get_current`, `get_voltage_diff`, `get_power`, `get_post_voltage`, `calculate_current` |
| Desenho | `draw`, `visual_state`, `set_visual_state` |
| Edição | `get_edit_info`, `set_edit_value`, `model_buttons`, `start_model_edit`, `slider` |
| Osciloscópio | `scope_value`, `scope_units`, `scope_values`, `can_view_in_scope`, `scope_source` |
| Estrutura | `child_elements` (composições: subcircuitos e afins) |
| Interação | `simple_toggle`, `toggle_key`, `link_label`, `is_momentary` |

`Base` guarda o que todo componente tem: posição (`x`, `y`, `x2`, `y2`), flags, tensões dos
terminais (`volts`), corrente, nós (`nodes`), seleção e destaque (`selected`, `highlight`,
`scope_role`).

O macro `impl_element_base!("NomeDaClasse")` gera o que é mecânico (`base`, `as_any`,
`class_name`…).

### 4.2 O registro (`elements/registry.rs`)

`ElementDescriptor` descreve um componente: o nome da classe, a **família** (o feature do
Cargo), o texto do menu, a categoria, a fábrica de colocação, o código de arquivo e o leitor de
arquivo, e opções (atalho, ícone, "rápido" na barra, escondido). `Registry::standard()` monta o
registro com todas as famílias compiladas (`elements::register_all`).

O registro responde:

| Pergunta | Método |
|----------|--------|
| Criar um componente para colocar | `create(id, x, y)` |
| Ler uma linha de arquivo | `create_from_dump(código, …)`, `get_by_dump` |
| Ler uma tag XML | `get_by_xml_tag` |
| Montar o menu Desenhar | `menu()` |
| Quais famílias existem | `families()` |

Códigos de arquivo duplicados ou nomes repetidos são erros do registro (`errors()`), que um
teste verifica.

### 4.3 Estado que o desenho precisa: `visual_state`

A janela tem **uma cópia dos componentes** que só recebe tensões, correntes e o `visual_state`
(uma lista de `f64`) a cada quadro. O que um componente desenha e que *não* é tensão ou corrente
(a posição de uma chave analógica, a temperatura de uma lâmpada, os segmentos de uma linha de
transmissão, a posição de um relé) precisa ir em `visual_state`/`set_visual_state`, senão a
janela mostra um valor parado. `tests/worker_draw.rs` confere isso para todos os 373 exemplos.

### 4.4 Composições

`elements/composite.rs` define `ChildNet`, que guarda os componentes internos de uma composição e
liga os terminais externos aos internos; `forward_to_net!` gera os métodos do trait que só
repassam. Subcircuitos (`subcircuit.rs`), o par Darlington, o OTA, o cristal e outros usam isso.
Um subcircuito é uma lista de tags XML num `CompositeModel`; cada instância constrói seus filhos a
partir dele. Limites de segurança contra modelos que se contêm: 16 níveis e 50 000 partes.

### 4.5 Modelos

Diodos, transistores, MOSFETs e relés têm modelos nomeados (`elements/models.rs`), guardados num
registro por thread e gravados no arquivo (tags `dm`, `tm`, `mm`, `rlm`). O diálogo de edição de
modelo é descrito em `model_edit.rs` (campos) e aplicado pela sessão.

## 5. Osciloscópios

`scope/mod.rs`:

- `Scope`: velocidade (passos por amostra), flags do arquivo, posição (coluna), escalas, lista
  de `ScopePlot`, gatilho (`Trigger`), modos (`xy`, `plot_xy`, `fft`), eixos do X/Y
  (`XyAxes`) e, se em X/Y, a imagem (`Trace`).
- `ScopePlot`: o componente e o valor plotado (`VAL_*`), as unidades, dois buffers circulares de
  2048 pontos (**mínimo e máximo** de cada amostra, porque vários passos cabem numa amostra), o
  filtro do acoplamento CA, a escala manual.
- `Scope::time_step` roda a cada passo da simulação: acumula mínimos e máximos, alimenta o
  gatilho e, em X/Y, pinta a imagem.
- `Scope::view` produz um `ScopeView`, a cópia imutável que a interface desenha. É isso que
  viaja nos snapshots.
- `Scope::from_line`/`from_xml` e `to_xml` leem e gravam o formato do original, com os mesmos
  bits de flag (64, 128, 1024, 65536, 1<<23…) e os atributos `xy2*` e `tp`.

`scope/trace.rs` guarda a imagem do X/Y: uma grade de 512 por 192 células RGB, pintada **a cada
passo** (por isso a imagem de exemplos como a carinha sorridente só funciona com todos os
passos), com esmaecimento ligado ao ritmo da tela. `scope/fft.rs` tem a FFT com janela senoidal,
os decibéis e o ângulo de fase.

Osciloscópios soltos no circuito são o componente `ScopeElm` (`scope_elm.rs`), que guarda as
configurações como o XML de uma tag `o` e desenha o `ScopeView` que recebe.

## 6. A aplicação

### 6.1 `Session` (`app/session.rs`, `edit.rs`, `file.rs`, …)

É **o documento em edição**, e não depende de GTK (por isso quase tudo é testado sem janela):

- a `Simulation` (os componentes editáveis);
- as opções, os sliders (`Adjustable`), o desfazer (uma pilha de textos XML do circuito), a área
  de transferência, a ferramenta atual e o arrasto em andamento;
- o enquadramento (`view`) e o tamanho do canvas;
- `Extras`: partes do arquivo que só são guardadas para gravar de volta;
- os osciloscópios na forma que vão ao arquivo.

O código está espalhado em arquivos por tema, todos `impl Session`:

| Arquivo | Tema |
|---------|------|
| `session.rs` | A estrutura, o ritmo da execução (`update`, `step_rate`), os sliders |
| `edit.rs` | Seleção, arrasto, copiar/colar, desfazer, zoom, osciloscópios, propriedades |
| `file.rs` | Abrir, salvar, autosave, ponto CC |
| `engine.rs` | A conversa com o trabalhador |
| `subcircuits.rs`, `contexts.rs`, `inspect.rs` | Criar subcircuitos, editar modelos, olhar dentro |
| `wires.rs` | Converter fios em fios roteados |
| `settings.rs` | As opções gravadas |

### 6.2 O trabalhador (`app/worker.rs`)

Uma thread que tem a **sua própria** `Session`. A interface manda `Command`s por um canal
(`Load`, `SetRunning`, `Reset`, `SetSpeed`, `Edit`, `FindDc`, `Audio`, …) e lê o último
`FrameSnapshot` de uma posição compartilhada. Como os componentes não são `Send`, o circuito
atravessa a fronteira **como texto XML** (que carrega o estado em execução).

Com um trabalhador ligado, a `Session` da janela é só o documento: a cada quadro,
`Session::update` chama `pump`, que copia o snapshot para os componentes dela (tensões,
correntes, `visual_state`, osciloscópios). Sem trabalhador (testes, uso sem janela), a própria
sessão dá os passos.

`FrameSnapshot` (`solver/snapshot.rs`) leva: o tempo, o passo, o estado (rodando, parado, erro),
as tensões dos nós, as dos terminais e as correntes de cada componente, o `visual_state` de cada
um, as visões dos osciloscópios (acoplados e soltos), e o erro.

### 6.3 Comandos e menus (`app/commands.rs`, `menus.rs`)

Todo item de menu, botão ou atalho é um `Cmd`. As tabelas `file_menu()`, `edit_menu()`,
`scopes_menu()`, `options_menu()`, `tools_menu()` e `draw_menu(registry)` descrevem os menus como
árvores de `Node`. `Cmd::enabled(&Session)` diz se está habilitado; `dispatch(&mut Session, &Cmd)`
executa o que só muda a sessão e devolve um `Outcome`:

- `Done`: acabou;
- `Clipboard(texto)`: a janela precisa colocar texto na área de transferência;
- `Ui(cmd)`: precisa de janela (diálogo, seletor de arquivo, impressão…).

`menus.rs` transforma as árvores em modelos do GTK, cria uma ação por comando, ajusta
habilitado e marcado (`sync_actions`) e instala os atalhos como **controlador de atalhos da
janela** (não da aplicação: `set_accels_for_action` custava mais de um segundo na abertura).

### 6.4 A janela (`app/window.rs`)

`App` é o componente relm4. `init` monta tudo: menus, barra, canvas (um `GtkGLArea` com
fallback para um `DrawingArea` do Cairo), a área dos osciloscópios, o painel. `update` trata as
mensagens `Msg`. Os eventos do mouse e do teclado chamam métodos da `Session`; **a janela não
tem regra de negócio**, só o encaminhamento. Diálogos ficam em `dialogs.rs`.

O laço de quadros (`add_tick_callback`) chama `Session::update(dt)`, atualiza a altura da área
dos osciloscópios, e pede o redesenho.

### 6.5 Medição (`app/bench.rs`)

`--bench-render ARQUIVO` roda fases (animar, mover, ampliar, clicar) e mede quadros por segundo,
tempo de desenho, tempo até o primeiro quadro e latência de clique. Veja
[tests/bench/README.md](../tests/bench/README.md).

## 7. Desenho

`render/painter.rs` define o trait `Painter`: cor, espessura, fonte, transformações, caminhos
(`move_to`, `line_to`, `arc`, `stroke`, `fill`), texto, `clip_rect`. Tudo é desenhado através dele.

Backends:

| Backend | Arquivo | Uso |
|---------|---------|-----|
| GPU | `gl.rs` (femtovg sobre o OpenGL do `GtkGLArea`, funções carregadas da libepoxy) | O canvas |
| CPU | `cairo.rs` | O canvas sem GL, PNG, impressão |
| SVG | `svg.rs` | Exportar SVG e os testes de desenho (`insta`) |

`gl.rs` é o **único** arquivo com `unsafe`. Ele dimensiona o buffer pelo viewport que o GTK
criou, e avisa quando o renderizador é de software.

`render/scene.rs` desenha o circuito (`draw_scene`: fundo, componentes, terminais, sobreposições)
e calcula o enquadramento (`View`). `render/scopes.rs` desenha os osciloscópios: o layout em
colunas (`layout`, `scope_at`), a grade, as curvas, as medidas, o espectro, o X/Y, a engrenagem, a
área de informação. `render/export.rs` faz PNG/SVG.

## 8. Como as peças trabalham juntas: três percursos

### 8.1 Abrir um circuito

`Cmd::Open` → seletor de arquivo → `Msg::OpenPath` → `Session::open_file` → `open_text` →
`parse` (erros de carga vão para `load_errors`) → `install` (cria a `Simulation`, os
osciloscópios, os sliders; reanalisa) → o trabalhador recebe `Command::Load{xml}` → ele analisa
a sua cópia e começa a rodar.

### 8.2 Um quadro com a simulação rodando

Trabalhador: `update(elapsed)` dá `step_rate × elapsed` passos (`run_steps`), calcula as
correntes dos fios, e a cada ~8 ms publica um `FrameSnapshot`.
Janela: o tick chama `Session::update` → `pump` → `apply_snapshot` copia tudo para os
componentes → `paint_frame` → `draw_scene` e `draw_scopes`.

### 8.3 Editar um valor

Clique duplo → `edit_fields(i)` → diálogo → `Msg::ApplyEdit` → `apply_edit_fields` → `push_undo`
(grava o XML antes) → `set_edit_value` no componente → se o campo só muda um número, o
trabalhador recebe `Command::Edit{elm, n, valor}` (sem reconstruir o circuito); se muda a
estrutura, `reanalyze` reenvia o circuito inteiro.

## 9. Famílias de componentes (features)

`Cargo.toml` tem um feature por família, todos ligados por padrão: `passive`, `reactive`,
`sources`, `switches`, `outputs`, `logic`, `diodes`, `analog`, `transistors`, `mosfets`,
`digital`, `subcircuits`. O feature `gui` liga o GTK, o relm4, o femtovg e o resto.

Regras:

- Cada família é compilada e testada **sem ela**, uma de cada vez, no CI (job `families`).
  Código que depende de uma família precisa estar atrás do mesmo feature: `#[cfg(feature =
  "…")]`.
- Dependências entre famílias que existem hoje: a entrada lógica (`logic`) usa o interruptor
  (`switches`); os relés precisam de `switches` e `reactive` (a bobina é um indutor); alguns
  componentes compostos usam `passive` e `reactive` para ajustar seus valores.
- Os testes que usam uma família têm `#[cfg(feature = "…")]` para cada família de que dependem.
- Um circuito que usa um componente de uma família ausente carrega com um erro `UnknownType` que
  dá o nome, e o resto carrega.
- O `gui` exige **todas** as famílias; a combinação não é testada.

## 10. Testes e verificação

| O quê | Onde |
|-------|------|
| Unidade | `#[cfg(test)]` nos próprios módulos |
| Solver contra resultado analítico | `tests/solver_basic.rs`, `us1_rlc.rs` |
| Todos os exemplos carregam e fazem ida e volta | `tests/compat_load.rs` |
| Resultados contra o CircuitJS1 | `tests/compat_golden.rs` com gravações em `tests/compat/golden/` |
| Casos extremos e fuzz | `tests/edge_cases.rs` (`FUZZ_SEED`, `FUZZ_ROUNDS`) |
| A janela recebe o estado certo | `tests/worker_draw.rs`, `tests/worker.rs` |
| Cobertura das famílias | `tests/inventory.rs` (todos os 153 tipos do original têm componente) |
| Modularidade (componente fora de `src/`) | `tests/modularity.rs` |
| Desenho | `tests/render_svg.rs` (com `insta`), `render_cairo.rs` |
| Osciloscópios | `tests/scopes.rs`, `src/render/scopes.rs` |
| Desempenho | `benches/solver.rs`, `--bench-render`, `tests/bench/` |

### Comparação com o original

`tests/compat/` tem 373 gravações (`golden/*.json`) feitas rodando cada exemplo no CircuitJS1
(`src/bin/golden.rs` abre uma cópia do aplicativo Electron). O teste roda o mesmo circuito aqui e
compara tensões e correntes em pontos de verificação, com tolerância. As diferenças aceitas
estão em `known_differences.json` (com o motivo), e os 7 circuitos não determinísticos em
`nondeterministic.json`. Os passos para regravar estão em `tests/compat/README.md`.

### Antes de dar uma mudança por pronta

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features gui -- -D warnings
cargo test --release
```

Para mudanças em famílias ou em código atrás de um `cfg`, compile também sem uma família:

```bash
cargo test --no-default-features --features passive,reactive,sources,switches,logic,diodes,analog,transistors,mosfets,digital,subcircuits   # sem `outputs`
```

## 11. Convenções do código

- **Sem `unwrap`, `expect`, `panic!` em código de produção** (avisos no `Cargo.toml`, erro no
  CI). Falhas são `Result` ou mensagens. Os testes podem.
- **Sem `unsafe`** fora de `render/gl.rs`. Cada `#[allow]` leva um comentário que o explica.
- **Toda função pública tem `///`** (`missing_docs` é aviso), e o crate tem exemplos executáveis
  em `src/lib.rs` e `src/solver/mod.rs`.
- **Cabeçalho de licença** em todo arquivo e a linha *Derived from CircuitJS1* nos traduzidos.
- **Lints de estilo desligados de propósito** (`needless_range_loop`, `too_many_arguments` e
  outros, no `Cargo.toml`), porque o porte fiel usa laços e assinaturas parecidos com os do Java
  para facilitar a comparação.
- **O texto da interface passa por `locale::ls`** (a chave é o texto em inglês do original).
  Textos novos precisam de entrada em `assets/locale/locale_pt.txt`.
- **Comentários** dizem *por que*, não *o quê*. Onde o código copia uma esquisitice do original,
  o comentário diz isso.

## 12. Armadilhas conhecidas

1. **A janela desenha de uma cópia.** Se algo "não anima" na janela mas funciona nos testes,
   quase sempre falta o estado em `visual_state` (seção 4.3).
2. **Um aplicativo já aberto engole o segundo.** O programa é um `GApplication` de instância
   única; rodar o binário de novo só repassa para a janela aberta e sai. Para medir ou testar
   com a janela, desligue o barramento: `DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent`.
3. **Atalhos pela aplicação são lentos.** Não use `set_accels_for_action`: cada chamada
   atualiza todos os itens de menu. Use o controlador de atalhos (`menus::install_shortcuts`).
4. **Famílias:** código novo fora de `src/elements` que cite um tipo de componente precisa de
   `#[cfg(feature)]`, ou o build sem essa família quebra.
5. **O osciloscópio X/Y precisa de todos os passos.** A imagem é pintada em
   `Scope::time_step`; não a mova para o desenho do quadro.
6. **Modelos de subcircuito e de dispositivo são globais da thread.** Testes que os definem
   usam nomes próprios, para não colidirem.
7. **O desfazer guarda o circuito inteiro como XML.** Chame `push_undo` *antes* de mudar.
8. **Os assets não vão dentro do binário:** os exemplos, traduções e ícones são lidos de
   `assets/` por caminho (`assets_dir()`), não embutidos no binário.
   O emblema do diálogo Sobre segue o mesmo caminho: `assets/icons/hicolor/scalable/apps/org.prometheuscs.App.svg`
   entra na busca de ícones do GTK e, se faltar, o diálogo mostra o ícone genérico (`src/app/icons.rs`).
9. **`set_scope_hover`/`apply_scope_roles` rodam a cada quadro** (em `refresh_draw_ctx`): são
   baratos, mas não ponha trabalho pesado ali.

## 13. Receitas: onde mexer para…

| Quero… | Mexo em… |
|--------|----------|
| Acrescentar um componente | Um arquivo em `src/elements/` + uma entrada em `register` (veja [adding-a-component.md](adding-a-component.md)) |
| Mudar como um componente se comporta na matriz | O `stamp`/`do_step` dele |
| Acrescentar um item de menu | A tabela do menu em `app/commands.rs` + o `Cmd` + `dispatch` (ou `window.rs` se precisa de janela) + a tradução |
| Acrescentar um atalho | O `accel` no `item(…)` do menu, ou `.shortcut('x')` no descritor |
| Acrescentar uma opção | Um campo em `AppSettings`, um `Toggle` em `edit.rs`, a linha em `options_menu()` e, se mudar o desenho, em `DrawCtx` |
| Acrescentar um valor plotável em osciloscópios | `scope_value`/`scope_units`/`scope_values` do componente |
| Mudar o desenho de um osciloscópio | `render/scopes.rs` (e um teste que renderiza para SVG) |
| Mudar o formato de arquivo | `format/` e, se for uma entrada nova de um componente, o `dump`/`undump_xml` dele; **rode `tests/compat_load.rs`** |
| Acrescentar um campo editável | `get_edit_info`/`set_edit_value` do componente (o diálogo é gerado) |
| Acrescentar uma tradução | `assets/locale/locale_<código>.txt`; o idioma novo entra em `LANGUAGES` (`src/locale.rs`) |
| Mudar o que a janela recebe do solver | `solver/snapshot.rs`, `Simulation::snapshot` e `Session::apply_snapshot` |
| Medir desempenho | `cargo bench --bench solver` e `--bench-render` |
| Conferir contra o original | `tests/compat/README.md` |
