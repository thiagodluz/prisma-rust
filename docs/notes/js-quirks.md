# Diferenças entre o motor Rust e o `engine.js`

O motor Rust joga **exatamente** como o `engine.js` do Prisma: `tests/cross_check.rs` repete
partidas inteiras geradas pelo JS e exige texto idêntico, rodada a rodada (jogada, pontos, nível,
próximo id, eventos e tabuleiro com ids). Já `tests/differential.rs` repete 20 partidas
ALEATÓRIAS do JS (2000 ações misturadas: dica, jogadas inválidas, cheats, desfazer, embaralhar e
saves certos e estragados) e compara cada ação por um "carimbo" do texto completo. Para refazer
os arquivos de referência, ou ver o texto do JS de uma ação que divergiu, veja o começo de
`tests/golden/generate-diff.mjs`. Este arquivo lista o que é diferente **de propósito**, para
ninguém se surpreender (e para a Fase 2 saber o que repetir no leitor de arquivos).

## Nomes e formato

- `move` do JS é `play` no Rust (`move` é uma palavra reservada).
- O `Set` do JS virou `Vec` com `contains`: a ordem em que as casas entram importa, e o `Set` do JS
  também guarda a ordem de inserção.
- Os eventos são o `enum Event`, e `falls[destino]` guarda a casa de origem da pedra. Um número
  negativo quer dizer "pedra nova que veio de cima do tabuleiro".
- `SavedGame` é o save **já lido**. Ler o arquivo (JSON) fica para a Fase 2.
- Os cheats recebem `i64` (pontos, nível e casa), como o JS recebe qualquer número, e devolvem
  `bool`; só `cheat_explode` e `cheat_supernova` devolvem um `MoveResult`. Já o `play` recebe
  `usize` e confere os limites sozinho.

## Limites de números

O JS aceita inteiros até 2^53 − 1. Os tipos do Rust são menores, então `restore` recusa (devolve
`false` e não muda nada) o que não cabe neles:

- **Id de pedra** num save: de 1 a 2.000.000.000. O contador de ids é `u32`, e esse limite deixa
  uns 2 bilhões de ids livres para pedras novas. Pelo mesmo motivo, `restore` também recusa se o
  próprio jogo já está com `next_id` acima desse número.
- **Nível**: até 4.294.967.295 (`u32`). Num save **sem versão** isso vale para o nível
  calculado, então ele é recusado a partir de cerca de 2,58 × 10^13 pontos.
- **`levelStartScore` negativo**: recusado (o Rust usa número sem sinal). O jogo nunca gera isso.
- **Nível máximo**: a subida de nível "para" em 4.294.967.295 (`saturating_add`) em vez de estourar.
  Nesse nível absurdo o início do nível deixa de bater com a conta, e um `to_saved` dali seria
  recusado por `restore`.

## Sorteios

- `pick` limita o índice sorteado a `len - 1`. Uma fonte de sorte que devolve exatamente `1.0`
  (ou `NaN`, ou algo fora de 0..1) nunca produz um índice fora do tabuleiro. O JS devolveria
  `undefined` nesses casos.
- Uma fonte de sorte **constante** (sempre `0.0`, por exemplo) faz a reação em cadeia nunca
  terminar. O JS também trava; não há limite de cascata.

## O gancho dos testes

`Game::force_no_moves` faz o papel do `game.hasMove = () => false` que os testes do JS fazem.
Não é para uso normal: com ele ligado, `new_game` e `restart` dão pânico (não existe tabuleiro
jogável) e todo embaralhar falha. No `restore`, o jogo-candidato ignora o gancho, como no JS
(lá o candidato nasce com `Object.create(Game.prototype)` e não herda a troca de método).

## Campos públicos

Como no JS, os campos de `Game` são públicos. Um chamador que escreva neles precisa respeitar as
invariantes descritas na documentação de `Game` (por exemplo, `level_start_score <= score`),
senão uma jogada pode dar pânico. Prefira `play`, `restore` e os `cheat_*`.

## O que o leitor de saves da Fase 2 precisa repetir do JS

`SavedGame` já é tipado, então estas checagens do JS passam a ser do leitor do arquivo, que
deve **recusar o save inteiro** se:

- o modo não for exatamente `"zen"`, `"classic"` ou `"endless"`;
- um número não for inteiro (pontos, nível, início de nível, deslocamento, id, cor);
- um booleano (`cheatsUsed`, `noDefeat`, `ended`) não for `true` ou `false` de verdade
  (o JS testa `=== true`: qualquer outra coisa conta como `false`);
- uma pedra tiver `type` desconhecido, ou `color` que não seja número (ou `null` no Espectro).

Também vale lembrar: `SavedGame.score` e os demais números são `i64`, e `restore` recusa o que
estiver fora do intervalo seguro, então o leitor pode passar o número lido sem medo de estouro.
