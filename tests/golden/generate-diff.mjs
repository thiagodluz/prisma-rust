// Gera tests/golden/differential.txt: partidas ALEATÓRIAS rodadas no engine.js ORIGINAL do Prisma.
//
// Uso (a partir da pasta do projeto Rust):
//   node tests/golden/generate-diff.mjs <caminho do engine.js> tests/golden
//   node tests/golden/generate-diff.mjs <caminho do engine.js> --detail <semente> <ação>
//
// Diferente do generate.mjs (que só joga a dica, rodada a rodada), aqui cada partida mistura
// jogadas da dica, jogadas em casas quaisquer (muitas inválidas), todos os cheats, desfazer,
// embaralhar e restaurar saves (versão 4, com uma trinca injetada, e o formato antigo só com
// números). O teste Rust (tests/differential.rs) repete as MESMAS ações com as mesmas fontes de
// sorte e compara.
//
// Para o arquivo não ficar com dezenas de megabytes, cada ação vira UMA linha:
//   <semente> <ação> <resumo> score=S level=L next_id=I undo=U #<carimbo>
// O carimbo é o FNV-1a de 64 bits do texto COMPLETO da ação (todos os eventos, todos os
// tabuleiros com ids, o estado final). Se o Rust divergir em qualquer detalhe, o carimbo muda.
// Com `--detail` este script imprime o texto completo de uma ação, para comparar com o do Rust.

import {writeFileSync} from 'node:fs';
import {join, resolve} from 'node:path';
import {pathToFileURL} from 'node:url';

const SEEDS = 20;
const STEPS = 100;

const args = process.argv.slice(2);
let wanted = null; // {seed, step} quando o pedido é só imprimir o detalhe de uma ação
const detailAt = args.indexOf('--detail');
if (detailAt >= 0) {
  wanted = {seed: Number(args[detailAt + 1]), step: Number(args[detailAt + 2])};
  args.splice(detailAt, 3);
}
const [enginePath, outputDir] = args;
if (!enginePath || (!outputDir && !wanted)) {
  console.error('uso: node generate-diff.mjs <engine.js> <pasta de saída> | --detail <semente> <ação>');
  process.exit(1);
}
const {Game} = await import(pathToFileURL(resolve(enginePath)).href);

// A mesma fonte de sorte do Rust (`lcg_random`).
const lcg = seed => () => ((seed = (1664525 * seed + 1013904223) >>> 0) / 2 ** 32);
const KINDS = {normal: 'n', burst: 'b', cross: 'c', spectrum: 's'};

// FNV-1a de 64 bits sobre os bytes do texto (que é só ASCII).
function fnv1a(text) {
  let hash = 0xcbf29ce484222325n;
  for (const byte of Buffer.from(text, 'utf8')) {
    hash ^= BigInt(byte);
    hash = BigInt.asUintN(64, hash * 0x100000001b3n);
  }
  return hash.toString(16).padStart(16, '0');
}

// Conta o que as ações exercitaram (só para o resumo impresso no fim; não vai para o arquivo).
const coverage = new Map();
const count = name => coverage.set(name, (coverage.get(name) ?? 0) + 1);

const tileText = tile => `${tile.id}:${tile.color === null ? '-' : tile.color}:${KINDS[tile.type]}`;
const boardText = board => board.map(row => row.map(tileText).join(' ')).join('|');

function activatedText(item) {
  count(`ativou ${item.type}`);
  if (item.type === 'supernova') return `supernova@${item.index}/${item.color}/${item.sources.join('.')}`;
  return `${item.type}@${item.index}#${item.id}`;
}

function eventText(event) {
  count(`evento ${event.type}`);
  switch (event.type) {
    case 'clear': {
      const created = event.creations.map(c => `${KINDS[c.type]}@${c.index}/${c.color}#${c.tile.id}`);
      return `clear c=${event.cells.join(',')} a=${event.activated.map(activatedText).join(',')} ` +
        `cr=${created.join(',')} ch=${event.chain} b=${event.base} cb=${event.creationBonus} ` +
        `m=${event.comboMultiplier} e=${event.earned} B=${boardText(event.board)}`;
    }
    case 'fall': return `fall ${event.falls.join(',')} ch=${event.chain}`;
    case 'shuffle': return `shuffle ch=${event.chain} B=${boardText(event.board)}`;
    case 'rescue': return `rescue ${event.index} ch=${event.chain}`;
    default: return `${event.type} ch=${event.chain}`;
  }
}

// A primeira linha é o resumo (vai para o arquivo); as seguintes são os eventos.
function resultText(result) {
  count(`jogada valid=${result.valid}`);
  if (!result.valid) return 'valid=false';
  if (result.levelsGained > 0) count('jogada subiu de nível');
  if (result.rescued) count('jogada com resgate');
  if (result.shuffled) count('jogada com embaralhada');
  if (result.ended) count('jogada que terminou o jogo');
  return `valid=true earned=${result.earned} chain=${result.chain} rescued=${result.rescued} ` +
    `shuffled=${result.shuffled} ended=${result.ended} lg=${result.levelsGained}\n  ` +
    result.events.map(eventText).join('\n  ');
}

const stateText = game =>
  `S score=${game.score} level=${game.level} lss=${game.levelStartScore} off=${game.progressionOffset} ` +
  `next=${game.nextId} mode=${game.mode} ended=${game.ended} cheats=${game.cheatsUsed} ` +
  `nd=${game.noDefeat} undo=${game.undoHistory.length} B=${boardText(game.board)}`;

const SPECIALS = ['burst', 'cross', 'spectrum'];

// Faz UMA ação aleatória no jogo e devolve o texto dela (a primeira linha é o resumo).
// A ordem dos sorteios (n(...)) é parte do contrato: o Rust tem que sortear na mesma ordem.
function doAction(game, n, seed) {
  const op = n(100);
  if (op < 55) { // jogar a dica (e recomeçar se não houver jogada possível)
    const hint = game.hint();
    if (!hint) { game.newGame(game.mode); return 'hint none restart'; }
    return `hint ${hint.a},${hint.b} ${resultText(game.move(hint.a, hint.b))}`;
  }
  if (op < 62) { const a = n(64), b = n(64); return `move ${a} ${b} ${resultText(game.move(a, b))}`; }
  if (op < 65) { const v = n(3000) + 1; return `addpts ${v} ${game.cheatAddPoints(v)}`; }
  if (op < 67) { const v = n(40) + 1; return `setlvl ${v} ${game.cheatSetLevel(v)}`; }
  if (op < 73) { const i = n(64), kind = SPECIALS[n(3)]; return `spawn ${i} ${kind} ${game.cheatSpawnSpecial(i, kind)}`; }
  if (op < 75) return `cshuffle ${game.cheatShuffle()}`;
  if (op < 77) { const on = n(2) === 1; return `nodefeat ${on} ${game.cheatNoDefeat(on)}`; }
  if (op < 79) return `explode ${resultText(game.cheatExplode())}`;
  if (op < 82) { const i = n(64); return `supernova ${i} ${resultText(game.cheatSupernova(i))}`; }
  if (op < 88) return `undo ${game.undoLastMove()}`;
  if (op < 91) return `shuffle ${game.shuffle(false)}`;
  if (op < 95) { // restaurar um save da versão 4, às vezes com uma trinca injetada
    const saved = {
      mode: game.mode, score: game.score, board: game.board.map(row => row.map(tile => ({...tile}))),
      progressionVersion: 4, level: game.level, levelStartScore: game.levelStartScore,
      progressionOffset: game.progressionOffset, cheatsUsed: game.cheatsUsed,
      noDefeat: game.noDefeat, ended: game.ended,
    };
    if (n(2) === 1) {
      const row = n(8), col = n(6), color = n(7);
      for (let k = 0; k < 3; k++) {
        if (game.board[row][col + k].type !== 'spectrum') saved.board[row][col + k] = {...saved.board[row][col + k], color};
      }
    }
    if (n(6) === 0) saved.ended = true; // às vezes o save diz "encerrado" (só vale no Clássico)
    // Às vezes o save é estragado de propósito, de um jeito que tem que ser recusado.
    const damage = n(12);
    if (damage === 0) saved.board[7][7] = {...saved.board[7][7], id: saved.board[0][0].id}; // id repetido
    else if (damage === 1) saved.level = game.level + 1; // nível que não bate com os pontos
    else if (damage === 2) { // cor fora de 0..6 (se a pedra for o Espectro, o save fica como está)
      if (saved.board[3][3].type !== 'spectrum') saved.board[3][3] = {...saved.board[3][3], color: 7};
    }
    else if (damage === 3) saved.levelStartScore = game.score + 1; // início do nível depois dos pontos
    else if (damage === 4) saved.progressionOffset = -1; // deslocamento negativo
    else if (damage === 5 && game.levelStartScore > 0) saved.score = game.levelStartScore - 1; // pontos abaixo do início do nível
    // Metade dos restores acontece no próprio jogo em andamento (confere o histórico de desfazer
    // e a fonte de sorte); a outra metade, num jogo novo.
    const live = n(2) === 1;
    const target = live ? game : new Game(lcg(seed + 1000));
    const ok = target.restore(saved);
    count(`restore v4 ${live ? 'no jogo vivo' : 'em jogo novo'} ${ok ? 'aceito' : 'recusado'}`);
    return `restore ${live ? 'live ' : ''}${ok}\n${stateText(target)}`;
  }
  // formato antigo (sem versão): só os números das cores
  const tiles = game.board.map(row => row.map(tile => tile.color === null ? 0 : tile.color));
  const mode = n(2) === 1 ? 'endless' : 'zen';
  const live = n(2) === 1;
  const target = live ? game : new Game(lcg(seed + 2000));
  const ok = target.restore({mode, score: game.score, board: tiles});
  count(`restore antigo ${live ? 'no jogo vivo' : 'em jogo novo'} ${ok ? 'aceito' : 'recusado'}`);
  return `legacy ${live ? 'live ' : ''}${ok}\n${stateText(target)}`;
}

const lines = [`# sementes ${SEEDS} ações ${STEPS}`];
for (let seed = 0; seed < SEEDS; seed++) {
  const game = new Game(lcg(seed));
  if (seed % 2 === 1) game.newGame('classic');
  const pick = lcg(seed * 7 + 3); // sorteia as ações; separada da sorte do jogo
  const n = k => Math.floor(pick() * k);
  for (let step = 0; step < STEPS; step++) {
    const text = `${doAction(game, n, seed)}\n${stateText(game)}`;
    if (wanted && wanted.seed === seed && wanted.step === step) {
      process.stdout.write(text + '\n');
      process.exit(0);
    }
    const summary = text.split('\n')[0];
    lines.push(`${seed} ${step} ${summary} score=${game.score} level=${game.level} ` +
      `next_id=${game.nextId} undo=${game.undoHistory.length} #${fnv1a(text)}`);
  }
}
if (wanted) { console.error('essa ação não existe'); process.exit(1); }
writeFileSync(join(outputDir, 'differential.txt'), lines.join('\n') + '\n');
console.log(`differential.txt: ${lines.length - 1} ações`);
console.log('cobertura:');
for (const [name, total] of [...coverage].sort()) console.log(`  ${name}: ${total}`);
