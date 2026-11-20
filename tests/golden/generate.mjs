// Gera os arquivos de referência ("golden") rodando o engine.js ORIGINAL do Prisma.
//
// Uso (a partir da pasta do projeto Rust):
//   node tests/golden/generate.mjs <caminho do engine.js> tests/golden
//
// Cada arquivo guarda uma partida inteira, rodada a rodada. O teste Rust (tests/cross_check.rs)
// repete a mesma partida e compara com este texto: se o Rust fizer qualquer coisa diferente do JS
// (um sorteio a mais, um id fora de ordem, uma pontuação diferente), o teste aponta a rodada.
//
// Formato de cada rodada (texto simples, sem JSON):
//   turn N a b earned E chain C score S level L next_id I events nome1,nome2,...
//   <8 linhas do tabuleiro, cada uma com 8 tokens id:cor:tipo>
// A cor é "-" para o Espectro; o tipo é n (normal), b (Pulso), c (Raio) ou s (Espectro).

import {writeFileSync} from 'node:fs';
import {join, resolve} from 'node:path';
import {pathToFileURL} from 'node:url';

const [enginePath, outputDir] = process.argv.slice(2);
if (!enginePath || !outputDir) {
  console.error('uso: node generate.mjs <caminho do engine.js> <pasta de saída>');
  process.exit(1);
}
const {Game} = await import(pathToFileURL(resolve(enginePath)).href);

// A mesma fonte de sorte do Rust (`lcg_random`) e dos testes do JS.
const lcg = seed => () => ((seed = (1664525 * seed + 1013904223) >>> 0) / 2 ** 32);
const KINDS = {normal: 'n', burst: 'b', cross: 'c', spectrum: 's'};

const boardLines = game => game.board.map(row =>
  row.map(tile => `${tile.id}:${tile.color === null ? '-' : tile.color}:${KINDS[tile.type]}`).join(' '));

function play(mode, seed, maxTurns) {
  const game = new Game(lcg(seed));
  if (mode === 'classic') game.newGame('classic');
  const lines = [];
  for (let turn = 1; turn <= maxTurns; turn++) {
    const hint = game.hint(); // a jogada vem da dica, o que também confere o hint do Rust
    if (!hint) break;
    const result = game.move(hint.a, hint.b);
    if (!result.valid) throw new Error(`a dica ${hint.a},${hint.b} não foi aceita na rodada ${turn}`);
    lines.push(`turn ${turn} ${hint.a} ${hint.b} earned ${result.earned} chain ${result.chain} ` +
      `score ${game.score} level ${game.level} next_id ${game.nextId} ` +
      `events ${result.events.map(event => event.type).join(',')}`);
    lines.push(...boardLines(game));
    if (game.ended) break;
  }
  return lines.join('\n') + '\n';
}

const scenarios = [
  {file: 'zen-7654321.txt', mode: 'zen', seed: 7654321, maxTurns: 100},
  {file: 'classic-42.txt', mode: 'classic', seed: 42, maxTurns: 300},
  // Esta semente faz o Zen travar uma vez, para conferir também o resgate (o Espectro criado).
  {file: 'zen-1.txt', mode: 'zen', seed: 1, maxTurns: 150},
];
for (const {file, mode, seed, maxTurns} of scenarios) {
  const text = play(mode, seed, maxTurns);
  writeFileSync(join(outputDir, file), text);
  console.log(`${file}: ${text.split('\n').filter(line => line.startsWith('turn ')).length} rodadas`);
}
