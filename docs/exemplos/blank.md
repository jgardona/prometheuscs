---
exemplo: blank.txt
titulo: Blank Circuit
categoria: Sem categoria
sem-verificacao: o circuito está em branco, sem componentes para medir
---

# Circuito em branco

<!-- navegacao:inicio -->
[Índice](README.md)
<!-- navegacao:fim -->

## Objetivo

Oferecer um **ponto de partida**: uma folha de circuito sem nenhum componente, para desenhar um circuito novo do zero. Não há nada a simular, e por isso este documento explica a folha e o que o simulador faz com ela.

## O circuito

O circuito não tem nenhum elemento. O arquivo só traz o cabeçalho com a configuração da simulação, entre elas o passo de tempo de $5\ \mu\text{s}$. A tabela de componentes abaixo está vazia porque não há componentes.

<!-- componentes:inicio -->
| N.º | Tipo | Elemento | Valores |
|----:|------|----------|---------|
<!-- componentes:fim -->

## Teoria

Um circuito sem elementos tem um sistema de equações vazio: nenhuma tensão de nó e nenhuma corrente para calcular. O simulador executa os passos de tempo sem trabalho e o relógio avança normalmente, a $5\ \mu\text{s}$ por passo. Ao acrescentar o primeiro componente, o simulador monta o sistema do circuito a partir dele, pelo método de análise nodal modificada, como em todos os outros exemplos.

## Resultados esperados

Nada além de uma folha em branco: sem formas de onda, sem valores nos terminais e sem osciloscópios. A carga deste exemplo não deve produzir erro nem aviso.

## O que observar

Carregue este exemplo e comece a desenhar o seu circuito sobre a folha em branco; o *Guia do usuário* descreve como acrescentar componentes e fios e como dar valores a eles.

## Limites do modelo

- A folha em branco não ensina um fenômeno físico; ela é uma ferramenta.
- O passo de $5\ \mu\text{s}$ do cabeçalho serve a circuitos de baixa frequência; circuitos muito rápidos pedem um passo menor, como os exemplos de linhas de transmissão deste projeto.

## Referências

Veja o *Guia do usuário* ([guia-do-usuario.md](../guia-do-usuario.md)) para o desenho de circuitos.

O circuito deste exemplo faz parte da coleção do simulador original de Paul Falstad e Iain Sharp (CircuitJS1); a explicação é deste projeto.
