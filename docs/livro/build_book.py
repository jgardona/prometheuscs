#!/usr/bin/env python3
"""Gera o livro em PDF com a documentação dos circuitos de exemplo.

    python3 docs/livro/build_book.py              # docs/livro-circuitos-de-exemplo.pdf
    python3 docs/livro/build_book.py --only ohms,jerk,cmosff   # teste rápido

Entrada: docs/exemplos/*.md (a ordem e as categorias vêm do índice docs/exemplos/README.md)
e os esquemas docs/exemplos/img/*.svg. Saída: um PDF feito com XeLaTeX. Ferramentas externas:
`xelatex` e `rsvg-convert`. Os arquivos intermediários ficam em `target/livro` (fora do git).

O conversor entende só o subconjunto de Markdown que os documentos usam (títulos, listas,
tabelas, imagens, matemática `$...$` e `$$...$$`, código e dois diagramas Mermaid). Qualquer
coisa que ele não reconheça aparece como aviso em vez de sumir calada.
"""
from __future__ import annotations

import argparse
import concurrent.futures
import datetime
import pathlib
import re
import shutil
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
EXEMPLOS = ROOT / "docs" / "exemplos"
SAIDA = ROOT / "docs" / "livro-circuitos-de-exemplo.pdf"

# Nome de cada categoria do menu em português (o nome original do menu vai entre parênteses).
CATEGORIAS = {
    "Basics": "Fundamentos",
    "A/C Circuits": "Circuitos de corrente alternada",
    "Passive Filters": "Filtros passivos",
    "Other Passive Circuits": "Outros circuitos passivos",
    "Diodes": "Diodos",
    "Op-Amps": "Amplificadores operacionais",
    "Transistors": "Transistores",
    "MOSFETs": "MOSFETs",
    "555 Timer Chip": "O temporizador 555",
    "Active Filters": "Filtros ativos",
    "Logic Families": "Famílias lógicas",
    "Combinational Logic": "Lógica combinacional",
    "Sequential Logic": "Lógica sequencial",
    "Analog/Digital": "Analógico e digital",
    "Power Converters": "Conversores de potência",
    "Phase-Locked Loops": "Malhas de captura de fase",
    "Transmission Lines": "Linhas de transmissão",
    "Controlled Sources": "Fontes controladas",
    "Subcircuits": "Subcircuitos",
    "Misc Devices": "Dispositivos diversos",
    "2-D Scope": "Osciloscópio 2-D",
}

AVISOS: list[str] = []


def aviso(msg: str) -> None:
    AVISOS.append(msg)


# ---------------------------------------------------------------------------------------------
# Texto corrido
# ---------------------------------------------------------------------------------------------

ESPECIAIS = {
    "\\": r"\textbackslash{}", "&": r"\&", "%": r"\%", "#": r"\#", "_": r"\_",
    "{": r"\{", "}": r"\}", "~": r"\textasciitilde{}", "^": r"\textasciicircum{}", "$": r"\$",
}
# caracteres que a fonte do texto não tem, ou que são melhores como símbolo matemático
TEXTO = {
    "\u03bc": "\u00b5",
    "\u03a9": r"\ensuremath{\Omega}", "\u2126": r"\ensuremath{\Omega}",
    "\u2192": r"\ensuremath{\rightarrow}", "\u2248": r"\ensuremath{\approx}",
    "\u2264": r"\ensuremath{\le}", "\u2265": r"\ensuremath{\ge}", "\u2212": "\u2013",
    "\u00a0": "~",
}
MATEMATICA = {
    "\u03bc": r"\mu ", "\u03a9": r"\Omega ", "\u2126": r"\Omega ", "\u00b7": r"\cdot ",
    "\u00d7": r"\times ", "\u00b1": r"\pm ", "\u2248": r"\approx ", "\u2192": r"\to ",
    "\u00b0": r"^\circ ", "\u2212": "-", "\u2013": "-", "\u2264": r"\le ", "\u2265": r"\ge ",
    "\u2260": r"\ne ", "\u221e": r"\infty ", "\u03c0": r"\pi ", "\u00a0": r"\ ",
}
CJK = re.compile("[\u3000-\u30ff\u4e00-\u9fff\uff00-\uffef]+")


def mapear_texto(s: str) -> str:
    s = "".join(TEXTO.get(c, c) for c in s)
    return CJK.sub(lambda m: "{\\cjkfont " + m.group(0) + "}", s)


def esc(t: str) -> str:
    return "".join(ESPECIAIS.get(c, c) for c in t)


# letras latinas acentuadas (de À a ÿ, sem × e ÷)
ACENTUADAS = "\u00c0-\u00d6\u00d8-\u00f6\u00f8-\u00ff"
PALAVRA_ACENTUADA = re.compile(r"[A-Za-z]*[" + ACENTUADAS + r"][A-Za-z" + ACENTUADAS + "]*")


def corrigir_matematica(m: str) -> str:
    """Troca os caracteres Unicode soltos de uma fórmula; dentro de \\text{...} ficam como texto."""
    saida = []
    for p in re.split(r"(\\text\{[^{}]*\})", m):
        if p.startswith("\\text{"):
            saida.append(mapear_texto(p))
            continue
        # uma palavra com letra acentuada fora de \\text (por exemplo V_{saída}) vira texto
        p = PALAVRA_ACENTUADA.sub(lambda w: "\\text{" + w.group(0) + "}", p)
        for parte in re.split(r"(\\text\{[^{}]*\})", p):
            if parte.startswith("\\text{"):
                saida.append(parte)
                continue
            for c in parte:
                if ord(c) <= 127:
                    saida.append(c)
                elif c in MATEMATICA:
                    saida.append(MATEMATICA[c])
                else:
                    aviso(f"caractere {c!r} (U+{ord(c):04X}) numa fórmula: {m[:60]}")
                    saida.append(c)
    return "".join(saida)


def codigo(t: str) -> str:
    e = mapear_texto(esc(t))
    for sep in (r"\_", "/", ".", "="):
        e = e.replace(sep, sep + r"\allowbreak{}")
    return r"\texttt{" + e + "}"


LABEL_INTERNO: set[str] = set()  # preenchido com os documentos que entram no livro


def inline(s: str) -> str:
    guardado: list[str] = []

    def guarda(tex: str) -> str:
        guardado.append(tex)
        return f"\x00{len(guardado) - 1}\x01"

    s = s.replace(r"\$", guarda(r"\$"))
    s = re.sub(r"`([^`]+)`", lambda m: guarda(codigo(m.group(1))), s)
    s = re.sub(r"\$\$(.+?)\$\$", lambda m: guarda("$\\displaystyle " + corrigir_matematica(m.group(1)) + "$"), s)
    s = re.sub(r"\$([^$]+?)\$", lambda m: guarda("$" + corrigir_matematica(m.group(1)) + "$"), s)

    def ligacao(m: re.Match) -> str:
        texto, alvo = m.group(1), m.group(2).strip()
        corpo = inline(texto)
        if re.match(r"https?://", alvo):
            return guarda(r"\href{" + alvo.replace("%", r"\%").replace("#", r"\#") + "}{" + corpo + "}")
        stem = alvo.split("#")[0].removesuffix(".md")
        if stem in LABEL_INTERNO:
            return guarda(r"\hyperref[ex:" + stem + "]{" + corpo + "}")
        return guarda(corpo)  # índice ou arquivo fora do livro: só o texto

    s = re.sub(r"\[([^\]]+)\]\(([^)]+)\)", ligacao, s)
    s = esc(s)
    s = re.sub(r"\*\*(?=\S)(.+?)(?<=\S)\*\*", r"\\textbf{\1}", s)
    s = re.sub(r"(?<![\w*])\*(?=\S)([^*]+?)(?<=\S)\*(?![\w*])", r"\\emph{\1}", s)
    s = re.sub(r'"([^"\n]*)"', "\u201c\\1\u201d", s)
    s = mapear_texto(s)
    s = re.sub(r"\x00(\d+)\x01", lambda m: guardado[int(m.group(1))], s)
    if "\x00" in s or "**" in s:
        aviso(f"marcação não resolvida: {s[:80]}")
    return s


def titulo_tex(texto: str) -> str:
    tex = inline(texto)
    if re.search(r"[$`*\\]", texto):
        simples = re.sub(r"\$[^$]*\$", "", texto).replace("`", "").replace("*", "")
        return r"\texorpdfstring{" + tex + "}{" + simples.strip() + "}"
    return tex


# ---------------------------------------------------------------------------------------------
# Tabelas
# ---------------------------------------------------------------------------------------------

def dividir_linha(linha: str) -> list[str]:
    """Divide uma linha de tabela nos `|`, sem mexer nos que estão dentro de fórmulas ou código."""
    def protege(m: re.Match) -> str:
        return m.group(0).replace("|", "\x02")

    linha = re.sub(r"`[^`]*`|\$[^$]*\$", protege, linha.strip())
    linha = linha.replace("\\|", "\x02")
    if linha.startswith("|"):
        linha = linha[1:]
    if linha.endswith("|"):
        linha = linha[:-1]
    return [c.strip().replace("\x02", "|") for c in linha.split("|")]


def largura_visivel(c: str) -> int:
    c = re.sub(r"\$[^$]*\$", "MMMMMM", c)
    codigo_ = sum(len(x) for x in re.findall(r"`([^`]*)`", c))  # a fonte de código é mais larga
    return int(len(re.sub(r"[`*]", "", c)) + codigo_ * 0.6)


def tabela(linhas: list[str], componentes: bool) -> str:
    cab = dividir_linha(linhas[0])
    alin = dividir_linha(linhas[1])
    corpo = [dividir_linha(l) for l in linhas[2:]]
    n = len(cab)
    corpo = [(r + [""] * n)[:n] for r in corpo]
    modo = []
    for a in alin:
        modo.append("r" if a.endswith(":") and not a.startswith(":") else "c" if a.startswith(":") and a.endswith(":") else "l")
    maximo = [max([largura_visivel(cab[j])] + [largura_visivel(r[j]) for r in corpo]) for j in range(n)]
    media = [max(1, sum(largura_visivel(r[j]) for r in corpo) / max(1, len(corpo))) for j in range(n)]
    largas = {j for j in range(n) if maximo[j] > 28}
    if componentes:
        largas = {n - 1}
    if not largas and sum(maximo) > 70:
        largas = {max(range(n), key=lambda j: maximo[j])}
    if largas:
        # colunas p{}: a largura de cada uma é proporcional ao texto que ela leva (limitado)
        pesos = [min(max(maximo[j], 4), 70) if j not in largas else min(max(media[j], 12), 70) for j in range(n)]
        soma = sum(pesos)
        espec = ""
        for j in range(n):
            larg = r"%.4f\dimexpr\linewidth-%d\tabcolsep\relax" % (pesos[j] / soma, 2 * n)
            ali = {"l": r"\raggedright", "r": r"\raggedleft", "c": r"\centering"}[modo[j]]
            espec += r">{" + ali + r"\arraybackslash}p{" + larg + "}"
    else:
        espec = "".join(modo)
    ambiente, abre = "longtable", r"\begin{longtable}[c]{" + espec + "}"
    cabecalho = " & ".join(r"\textbf{" + inline(c) + "}" if c else "" for c in cab) + r" \\"
    out = [r"\begingroup\small\setlength{\tabcolsep}{4pt}\renewcommand{\arraystretch}{1.15}", abre,
           r"\toprule", cabecalho, r"\midrule", r"\endhead"]
    for r in corpo:
        out.append(" & ".join(inline(c) for c in r) + r" \\")
    out += [r"\bottomrule", r"\end{" + ambiente + "}", r"\endgroup", r"\par\smallskip"]
    return "\n".join(out)


# ---------------------------------------------------------------------------------------------
# Diagramas Mermaid dos dois documentos que os usam (desenhados à mão em TikZ)
# ---------------------------------------------------------------------------------------------

TIKZ = {
    "cmosff": r"""
\begin{center}\adjustbox{max width=\linewidth}{%
\begin{tikzpicture}[>=Stealth,font=\small,
  estado/.style={draw,rounded corners=3pt,fill=PCblue!8,minimum width=2.2cm,minimum height=1cm}]
\node[estado] (q1) {$Q = 1$};
\node[estado,right=5.5cm of q1] (q0) {$Q = 0$};
\draw[->] (q1.20) to[bend left=20] node[above]{pressiona R} (q0.160);
\draw[->] (q0.200) to[bend left=20] node[below]{pressiona S} (q1.340);
\draw[->] (q1) edge[loop left,looseness=7] node[left,align=center]{pressiona S\\ou solta} (q1);
\draw[->] (q0) edge[loop right,looseness=7] node[right,align=center]{pressiona R\\ou solta} (q0);
\end{tikzpicture}}\end{center}
""",
    "cmosmsff": r"""
\begin{center}\adjustbox{max width=\linewidth}{%
\begin{tikzpicture}[>=Stealth,font=\small,node distance=0.55cm,
  caixa/.style={draw,rounded corners=3pt,fill=PCblue!8,minimum height=1.1cm,text width=2.1cm,align=center}]
\node (d) {$D$};
\node[caixa,right=of d] (s1) {chave: relógio alto};
\node[caixa,right=of s1] (m) {latch mestre};
\node[caixa,right=of m] (s2) {chave: relógio baixo};
\node[caixa,right=of s2] (e) {latch escravo};
\node[right=of e] (q) {$Q$};
\draw[->] (d) -- (s1); \draw[->] (s1) -- (m); \draw[->] (m) -- (s2);
\draw[->] (s2) -- (e); \draw[->] (e) -- (q);
\end{tikzpicture}}\end{center}
""",
}


# ---------------------------------------------------------------------------------------------
# Um documento
# ---------------------------------------------------------------------------------------------

def ler_documento(stem: str) -> tuple[dict[str, str], list[str]]:
    texto = (EXEMPLOS / f"{stem}.md").read_text(encoding="utf-8").split("\n")
    meta: dict[str, str] = {}
    if texto and texto[0].strip() == "---":
        fim = texto.index("---", 1)
        for l in texto[1:fim]:
            m = re.match(r"([a-z-]+):\s*(.*)", l)
            if m:
                meta[m.group(1)] = m.group(2).strip()
        texto = texto[fim + 1:]
    return meta, texto


RE_ITEM = re.compile(r"^(\s*)([-*]|\d+\.)\s+(.*)$")
RE_IMAGEM = re.compile(r"^!\[[^\]]*\]\(([^)]+)\)\s*$")


def inicio_de_bloco(l: str) -> bool:
    s = l.strip()
    return (s.startswith(("#", "|", "$$", "```", "<!--", ">")) or bool(RE_ITEM.match(l)) or bool(RE_IMAGEM.match(s)))


def lista(linhas: list[str], i: int) -> tuple[str, int]:
    itens: list[tuple[int, bool, list[str]]] = []  # recuo, numerada, linhas
    n = len(linhas)
    while i < n:
        l = linhas[i]
        m = RE_ITEM.match(l)
        if m:
            itens.append((len(m.group(1)), m.group(2)[0].isdigit(), [m.group(3)]))
            i += 1
        elif l.strip() == "":
            j = i
            while j < n and linhas[j].strip() == "":
                j += 1
            if j < n and RE_ITEM.match(linhas[j]):
                i = j
            else:
                break
        elif itens and not inicio_de_bloco(l):
            itens[-1][2].append(l.strip())
            i += 1
        else:
            break

    def emite(pos: int, recuo: int) -> tuple[str, int]:
        numerada = itens[pos][1]
        amb = "enumerate" if numerada else "itemize"
        out = [r"\begin{" + amb + "}"]
        while pos < len(itens) and itens[pos][0] >= recuo:
            if itens[pos][0] > recuo:
                sub, pos = emite(pos, itens[pos][0])
                out.append(sub)
                continue
            out.append(r"\item " + inline(" ".join(itens[pos][2])))
            pos += 1
        out.append(r"\end{" + amb + "}")
        return "\n".join(out), pos

    tex, _ = emite(0, itens[0][0])
    return tex, i


def converter(stem: str, rotulo: bool = True) -> str:
    meta, linhas = ler_documento(stem)
    out: list[str] = []
    i, n = 0, len(linhas)
    componentes = False
    mermaid_n = 0
    while i < n:
        l = linhas[i]
        s = l.strip()
        if s == "":
            i += 1
            continue
        if s.startswith("<!--"):
            if "navegacao:inicio" in s:
                while i < n and "navegacao:fim" not in linhas[i]:
                    i += 1
            elif "componentes:inicio" in s:
                componentes = True
            elif "componentes:fim" in s:
                componentes = False
            i += 1
            continue
        m = re.match(r"^(#{1,6})\s+(.*)$", s)
        if m:
            nivel, texto = len(m.group(1)), m.group(2)
            if nivel == 1:
                out.append(r"\Needspace{12\baselineskip}")
                out.append(r"\section{" + titulo_tex(texto) + "}" + (r"\label{ex:" + stem + "}" if rotulo else ""))
                cat = meta.get("categoria", "").replace(" > ", " \u203a ")
                out.append(r"{\small\itshape\color{black!65} Menu Circuits › " + esc(cat) + " › " +
                           esc(meta.get("titulo", "")) + r" \quad\textperiodcentered\quad arquivo " +
                           codigo(meta.get("exemplo", "")) + r"\par}\medskip")
            elif nivel == 2:
                out.append(r"\subsection*{" + titulo_tex(texto) + "}")
            elif nivel == 3:
                out.append(r"\subsubsection*{" + titulo_tex(texto) + "}")
            else:
                out.append(r"\paragraph*{" + titulo_tex(texto) + "}")
            i += 1
            continue
        if s.startswith("```"):
            lang = s[3:].strip()
            i += 1
            bloco = []
            while i < n and not linhas[i].strip().startswith("```"):
                bloco.append(linhas[i])
                i += 1
            i += 1
            if lang == "mermaid":
                if stem in TIKZ:
                    out.append(TIKZ[stem])
                else:
                    aviso(f"{stem}: diagrama Mermaid sem versão em TikZ")
            else:
                out.append(r"\begin{Verbatim}[fontsize=\footnotesize,xleftmargin=1.5em]")
                out.extend(bloco)
                out.append(r"\end{Verbatim}")
            continue
        if s.startswith("$$"):
            partes = [s]
            while partes[-1].count("$$") + sum(p.count("$$") for p in partes[:-1]) < 2:
                i += 1
                if i >= n:
                    aviso(f"{stem}: fórmula sem fechamento")
                    break
                partes.append(linhas[i].strip())
            i += 1
            formula = " ".join(partes)
            formula = re.sub(r"^\$\$|\$\$$", "", formula.strip()).strip()
            out.append("\\[\\adjustbox{max width=\\linewidth}{$\\displaystyle " + corrigir_matematica(formula) + "$}\\]")
            continue
        im = RE_IMAGEM.match(s)
        if im:
            alvo = im.group(1)
            nome = pathlib.PurePosixPath(alvo).stem
            out.append(r"\begin{center}\nopagebreak\adjustbox{max width=\linewidth,max height=0.46\textheight}"
                       r"{\includegraphics[scale=1.2]{fig/" + nome + r"}}\end{center}")
            i += 1
            continue
        if s.startswith("|"):
            bloco = []
            while i < n and linhas[i].strip().startswith("|"):
                bloco.append(linhas[i])
                i += 1
            if len(bloco) < 2:
                aviso(f"{stem}: tabela com menos de duas linhas")
                continue
            out.append(tabela(bloco, componentes))
            continue
        if RE_ITEM.match(l):
            tex, i = lista(linhas, i)
            out.append(tex)
            continue
        if s.startswith(">"):
            out.append(r"\begin{quote}" + inline(s.lstrip("> ")) + r"\end{quote}")
            i += 1
            continue
        par = [s]
        i += 1
        while i < n and linhas[i].strip() != "" and not inicio_de_bloco(linhas[i]):
            par.append(linhas[i].strip())
            i += 1
        out.append(inline(" ".join(par)) + "\n")
    return "\n".join(out) + "\n"


# ---------------------------------------------------------------------------------------------
# Índice, capítulos e montagem
# ---------------------------------------------------------------------------------------------

def ler_indice() -> list[tuple[str, list[str]]]:
    capitulos: list[tuple[str, list[str]]] = []
    for l in (EXEMPLOS / "README.md").read_text(encoding="utf-8").split("\n"):
        m = re.match(r"^## (.+?) \u2014 ", l)
        if m:
            capitulos.append((m.group(1), []))
            continue
        m = re.match(r"^\s*- \[[^\]]+\]\(([^)]+)\.md\)", l)
        if m and capitulos:
            capitulos[-1][1].append(m.group(1))
    return capitulos


def ler_titulos() -> dict[str, str]:
    t: dict[str, str] = {}
    for l in (EXEMPLOS / "README.md").read_text(encoding="utf-8").split("\n"):
        m = re.match(r"^\s*- \[([^\]]+)\]\(([^)]+)\.md\)", l)
        if m:
            t[m.group(2)] = m.group(1)
    return t


PREAMBULO = r"""\documentclass[11pt,a4paper,twoside,openany]{book}
\usepackage[a4paper,inner=2.7cm,outer=2.3cm,top=2.5cm,bottom=2.7cm]{geometry}
\usepackage{amsmath,amssymb,mathtools,bm}
\usepackage{fontspec}
\defaultfontfeatures{Ligatures=TeX}
\setmainfont{Latin Modern Roman}
\setsansfont{Latin Modern Sans}
\setmonofont{Latin Modern Mono}[Scale=MatchLowercase]
\newfontfamily\cjkfont{Droid Sans Fallback}
\usepackage{polyglossia}
\setmainlanguage{brazil}
\usepackage{microtype}
\usepackage{xcolor}
\definecolor{PCblue}{HTML}{1F4E79}
\usepackage{booktabs,array,longtable,adjustbox,needspace,fvextra,enumitem,titlesec,fancyhdr}
\usepackage{tikz}
\usetikzlibrary{arrows.meta,positioning}
\usepackage[hidelinks,colorlinks=true,linkcolor=PCblue,urlcolor=PCblue,citecolor=PCblue,
  pdftitle={Circuitos de exemplo do PrometheusCS},pdfauthor={Projeto PrometheusCS},pdflang={pt-BR}]{hyperref}
\setlength{\parindent}{0pt}
\setlength{\parskip}{0.55em plus 0.15em}
\setlength{\emergencystretch}{3em}
\raggedbottom
\setlist{leftmargin=1.6em,itemsep=0.15em,topsep=0.2em,parsep=0.15em}
\setcounter{secnumdepth}{1}
\setcounter{tocdepth}{1}
\titleformat{\chapter}[display]{\normalfont\bfseries\color{PCblue}}{\LARGE\chaptertitlename\ \thechapter}{0.6em}{\Huge}
\titleformat{\section}{\normalfont\Large\bfseries}{\thesection}{0.7em}{}[{\color{PCblue}\titlerule}]
\titlespacing*{\section}{0pt}{2.2ex plus 0.6ex}{1.2ex}
\titleformat{\subsection}{\normalfont\large\bfseries\color{PCblue}}{}{0pt}{}
\titlespacing*{\subsection}{0pt}{1.6ex plus 0.5ex minus 0.2ex}{0.5ex}
\pagestyle{fancy}
\fancyhf{}
\fancyhead[LE]{\small\nouppercase{\leftmark}}
\fancyhead[RO]{\small\nouppercase{\rightmark}}
\fancyfoot[C]{\small\thepage}
\renewcommand{\headrulewidth}{0.3pt}
\fancypagestyle{plain}{\fancyhf{}\fancyfoot[C]{\small\thepage}\renewcommand{\headrulewidth}{0pt}}
"""

CAPA = r"""\begin{document}
\frontmatter
\begin{titlepage}
\thispagestyle{empty}
\begin{center}
\vspace*{1.2cm}
{\fontsize{34}{40}\selectfont\bfseries\color{PCblue} Circuitos de Exemplo\par}
\vspace{0.4cm}
{\fontsize{22}{26}\selectfont\color{PCblue} do PrometheusCS\par}
\vspace{0.8cm}
{\large Objetivo, teoria, resultados esperados e o que observar\\ em %(n)d circuitos do simulador\par}
\vspace{1.0cm}
%(capa)s
\vfill
{\small Gerado em %(data)s a partir de \texttt{docs/exemplos} (commit \texttt{%(hash)s})\par}
\end{center}
\end{titlepage}
\chapter*{Sobre este livro}
Este livro reúne, num só volume, a documentação dos %(n)d circuitos do menu \emph{Circuits} do
PrometheusCS, um simulador de circuitos eletrônicos para o desktop escrito em Rust, que reescreve o
CircuitJS1. Cada exemplo é explicado por categoria: o objetivo, o circuito, a teoria com as
equações, os resultados esperados e o que observar no simulador.

\subsection*{Como cada exemplo está organizado}
\begin{itemize}
\item \textbf{Objetivo}: o que o circuito quer mostrar.
\item \textbf{O circuito}: o esquema, como o simulador o desenha, uma descrição de como as partes se
  ligam e a tabela de componentes. Na tabela, \emph{N.º} é a posição do elemento no arquivo do
  exemplo, \emph{Tipo} é o nome interno do elemento no simulador e \emph{Valores} traz os parâmetros
  dele. No esquema, a cor dos fios mostra a tensão e os pontos amarelos mostram a corrente.
\item \textbf{Teoria}: as equações que explicam o comportamento.
\item \textbf{Resultados esperados}: os valores e as formas de onda que o simulador deve dar. Em
  %(com)d dos %(n)d exemplos esses valores são conferidos automaticamente contra o simulador pela
  ferramenta \texttt{exdocs}; nos demais o próprio texto explica por que não há verificação numérica.
\item \textbf{O que observar}: o que fazer e olhar na janela do simulador.
\item \textbf{Limites do modelo}: onde o modelo do simulador se afasta do componente real.
\item \textbf{Referências}: leituras para aprofundar.
\end{itemize}

\subsection*{Origem e licença}
Os circuitos vêm da coleção do simulador original de Paul Falstad e Iain Sharp (CircuitJS1); a
explicação é deste projeto. O PrometheusCS é distribuído sob a licença GPL-2.0-or-later, os mesmos
termos do CircuitJS1.

\tableofcontents
\mainmatter
"""


def converter_svgs(stems: list[str], destino: pathlib.Path) -> None:
    destino.mkdir(parents=True, exist_ok=True)

    def um(stem: str) -> None:
        svg = EXEMPLOS / "img" / f"{stem}.svg"
        pdf = destino / f"{stem}.pdf"
        if not svg.exists():
            aviso(f"{stem}: sem esquema {svg.name}")
            return
        if pdf.exists() and pdf.stat().st_mtime >= svg.stat().st_mtime:
            return
        subprocess.run(["rsvg-convert", "-f", "pdf", "-o", str(pdf), str(svg)], check=True)

    with concurrent.futures.ThreadPoolExecutor(8) as ex:
        list(ex.map(um, stems))


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--only", help="lista de documentos separados por vírgula (teste rápido)")
    ap.add_argument("--work", default=str(ROOT / "target" / "livro"), help="pasta dos intermediários")
    ap.add_argument("--out", default=str(SAIDA), help="PDF de saída")
    ap.add_argument("--tex-only", action="store_true", help="só gera os .tex, sem compilar")
    args = ap.parse_args()

    indice = ler_indice()
    todos = [s for _, l in indice for s in l]
    em_disco = {p.stem for p in EXEMPLOS.glob("*.md")} - {"README"}
    if set(todos) != em_disco:
        print("o índice e os arquivos não conferem:", sorted(set(todos) ^ em_disco)[:10], file=sys.stderr)
        return 1
    if args.only:
        quer = set(args.only.split(","))
        indice = [(c, [s for s in l if s in quer]) for c, l in indice]
        indice = [(c, l) for c, l in indice if l]
    usados = list(dict.fromkeys(s for _, l in indice for s in l))  # um exemplo pode estar em duas categorias
    LABEL_INTERNO.update(usados)

    work = pathlib.Path(args.work)
    work.mkdir(parents=True, exist_ok=True)
    converter_svgs(usados, work / "fig")

    com = sum(1 for s in usados if "verificacoes" in ler_documento(s)[0])
    capa = ""
    thumb = ROOT / "assets" / "thumbnail.png"
    if thumb.exists():
        shutil.copy(thumb, work / "capa.png")
        capa = r"\includegraphics[width=0.92\textwidth]{capa.png}"

    corpo = []
    ja_incluidos: set[str] = set()
    titulos = ler_titulos()
    for k, (cat, stems) in enumerate(indice):
        nome = CATEGORIAS.get(cat)
        if nome is None:
            aviso(f"categoria sem tradução: {cat}")
        titulo = f"{nome} ({cat})" if nome and nome != cat else cat
        arq = f"cap{k + 1:02d}.tex"
        partes = [r"\chapter{" + esc(titulo) + "}\n"]
        for s in stems:
            if s in ja_incluidos:
                partes.append(
                    r"\begin{center}\small\emph{O exemplo “" + esc(titulos.get(s, s)) +
                    "” também está nesta categoria do menu; veja a seção \\ref{ex:" + s +
                    "}, na página \\pageref{ex:" + s + "}.}\\end{center}")
                continue
            ja_incluidos.add(s)
            partes.append(converter(s))
        (work / arq).write_text("\n".join(partes), encoding="utf-8")
        corpo.append(r"\input{" + arq + "}")

    try:
        h = subprocess.run(["git", "-C", str(ROOT), "rev-parse", "--short", "HEAD"], capture_output=True,
                           text=True, check=True).stdout.strip()
    except Exception:
        h = "?"
    dados = {"n": len(usados), "com": com, "capa": capa, "hash": h,
             "data": datetime.date.today().strftime("%d/%m/%Y")}
    (work / "livro.tex").write_text(PREAMBULO + (CAPA % dados) + "\n".join(corpo) + "\n\\end{document}\n",
                                    encoding="utf-8")
    for a in AVISOS:
        print("aviso:", a, file=sys.stderr)
    if args.tex_only:
        return 0

    for passo in (1, 2, 3):
        r = subprocess.run(["xelatex", "-interaction=nonstopmode", "-halt-on-error", "livro.tex"], cwd=work,
                           capture_output=True, text=True)
        if r.returncode != 0:
            print((work / "livro.log").read_text(errors="replace")[-3000:], file=sys.stderr)
            print(f"xelatex falhou no passo {passo}; veja {work / 'livro.log'}", file=sys.stderr)
            return 1
        if passo >= 2 and "Rerun to get" not in (work / "livro.log").read_text(errors="replace"):
            break
    saida = pathlib.Path(args.out)
    saida.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy(work / "livro.pdf", saida)
    print(f"{saida} ({saida.stat().st_size / 1e6:.1f} MB)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
