# Prisma em Rust — Fase 1: o engine

Data: 2026-11-19 · Status: **rascunho aguardando aprovação do Thiago** · Tamanho: projeto

## Intenção e por quê

O Thiago quer refazer o **Prisma** (jogo de combinar pedras, repositório `thiagodluz/Prisma`, versão 1.4.2, navegador + Android) em **Rust**, por **diversão e aprendizado**. O plano tem duas fases, planejadas uma de cada vez:

- **Fase 1 (este documento):** o "cérebro" do jogo, o `engine.js`, em Rust.
- **Fase 2 (planejada depois):** o resto — tela, toque, som, salvamento, Android — completando o jogo todo em Rust.

O Thiago tem conhecimento básico de programação. Quer explicações simples e **aprovar cada ação** antes de ela acontecer.

## Objetivos

1. Uma biblioteca Rust (`prisma_engine`) que **se comporta como o `engine.js`**: regras de combinação, pedras especiais (Pulso, Raio, Espectro, Supernova), pontos e níveis, modos Zen e Clássico, dica, embaralhar, resgate, desfazer, restaurar/migrar saves e cheats de desenvolvedor.
2. **Prova de que se comporta igual:** os 29 testes de `tests/engine.test.js` e os 7 de `tests/developer-cheats.test.js` portados para Rust e passando com `cargo test`.
3. **Didático:** código em Rust de verdade (não tradução linha por linha), com comentários em português explicando cada conceito de Rust na primeira vez que ele aparece, em linguagem simples.
4. **Pronta para a fase 2:** sem tela, sem arquivos, sem rede, sem nada preso a uma plataforma, para ser reaproveitada sem refazer.

## Não-objetivos

- Tela, toque, som, animação, `localStorage`/JSON, Android, WebAssembly, publicação: tudo isso é fase 2.
- Não usar o rust-anything (ele não cobre jogos nem navegador; os testes existentes fazem o papel de prova).
- Não alterar o repositório Prisma (acesso só de leitura).
- Não "melhorar" regras. Primeiro, comportamento igual. Se aparecer um bug no JS, ele é **anotado** para o Thiago decidir, não corrigido por conta própria.

## Anti-objetivos (seria falha mesmo que compile)

- Um Rust que o Thiago não consegue ler e entender.
- Testes alterados para passar em vez de corrigir o comportamento.
- Dependências externas sem necessidade.

## Restrições

- Rust 1.97 e Node 22 já instalados no ambiente (o Node só serve para a etapa 6). **Zero dependências** em Rust: só a biblioteca padrão (`std`).
- O código vive em `/home/claude/prisma-rust`, fora do repositório Prisma. Esse ambiente **não é permanente**: o código só fica seguro depois de enviado a um repositório do Thiago no GitHub (ver "Em aberto").
- A referência de comportamento é o `engine.js` do commit `94ae2437452dd906f337ab8e559f5e4e2a5b4f08` do Prisma. Em caso de dúvida, vale o que o JS faz hoje.
- Cada etapa só começa com a aprovação do Thiago.

## Abordagens consideradas

- **A) Só Rust, com os testes portados — ESCOLHIDA pelo Thiago.** Foco em aprender Rust. O resultado ainda não aparece no jogo.
- **B) Rust ligado ao jogo atual por WebAssembly — adiada.** Daria o prazer de jogar com o cérebro novo, mas coloca cedo a parte mais difícil (ligar Rust com JavaScript) antes do básico da linguagem. Pode voltar na fase 2.
- **Descartada:** usar o rust-anything (veja "Não-objetivos").

## Desenho

### Modelo de dados (JS → Rust)

| No JS | Em Rust |
|---|---|
| pedra `{id, color, type}` | `Tile { id: u32, color: Option<u8>, kind: TileKind }` |
| `type`: normal, burst (Pulso), cross (Raio), spectrum (Espectro) | `enum TileKind { Normal, Burst, Cross, Spectrum }` (Espectro não tem cor: `color = None`) |
| tabuleiro 8×8, índice = linha×8 + coluna (0 a 63) | igual; mesma numeração de índices |
| modo `'zen'`, `'classic'` (`'endless'` só em saves antigos, vira Clássico ao restaurar) | `enum Mode { Zen, Classic }` |
| eventos `{type: 'clear' \| 'fall' \| 'shuffle' \| 'gameover' \| 'rescue', ...}` | `enum Event { Clear {...}, Fall {...}, Shuffle {...}, GameOver {...}, Rescue {...} }` |
| ativações `{index, type, id}` e supernova `{index, color, sources}` | `enum Activation { Special {...}, Supernova {...} }` |
| resultado de `move` | `struct MoveResult { valid, events, earned, chain, rescued, shuffled, ended, levels_gained }` |
| save (JSON solto) | `struct SavedGame` montada em Rust; **sem JSON nesta fase** (JSON entra na fase 2) |

Os campos que o app mexe diretamente (`board`, `score`, `level`, `mode`, `ended`...) ficam públicos, como no JS.

### Aleatoriedade

O jogo recebe uma "fonte de sorte" (uma função que devolve um número de 0 a 1), como o `Game(random)` do JS. Assim os testes são repetíveis.

- `Game::new()` usa um gerador simples interno, semeado pela hora do sistema.
- `Game::with_random(f)` recebe a fonte de fora (testes).
- Um auxiliar de teste reproduz o gerador dos testes JS (`seed = seed × 1664525 + 1013904223` módulo 2³², dividido por 2³²), para sair **exatamente os mesmos números** que no JS.

### Etapas (cada uma termina com testes passando e é aprovada pelo Thiago)

1. **Núcleo:** tipos, tabuleiro, novo jogo, combinações (`matches`), `adjacent`, `swap`, `has_move`, `hint`. Testes: tabuleiros iniciais, jogada inválida, dica.
2. **Jogada:** criação de especiais, supernovas, `move` e a resolução em cascata (queda, pontos, níveis). Testes: 4/5/L-T, cadeias de Pulso e Raio, Espectro, Supernova, pontuação, metas de nível.
3. **Embaralhar, resgate, fim do Clássico, desfazer.** Testes correspondentes.
4. **Restaurar save:** migração das versões 2 a 4, saves antigos só com números, saves corrompidos (tudo ou nada: se o save é inválido, nada muda).
5. **Cheats de desenvolvedor** e os 7 testes de `developer-cheats.test.js`.
6. **Verificação cruzada com o JS (extra, no lugar do rust-anything):** um script Node usa o `engine.js` original, com semente fixa, e grava o resultado de várias jogadas (tabuleiro, pontos, eventos). Um teste Rust repete a mesma sequência e compara.

Ordem: 1 → 2 → 3. A etapa 4 depende de 1 e 3 (usa embaralhar/resgate). A 5 depende de 2 e 3. A 6 pode vir assim que a 2 estiver pronta.

### Armadilhas conhecidas (JS → Rust)

- **Ordem dos elementos:** o `Set` do JS guarda a ordem de inserção; o `HashSet` do Rust não. Onde a ordem aparece no resultado (`cells`, `activated`, `creations`, fila de ativação), usar `Vec`.
- **Testes que trocam funções em tempo de execução:** dois testes do JS substituem `hasMove` por "sempre sem jogadas" e um troca `random` por "sempre 0". Em Rust isso não existe: a fonte de sorte já é injetável, mas o caso "sem jogadas" precisa de um gancho de teste. A decisão fica para a etapa 3 e **não pode ser deixada de fora**, porque dois testes dependem dela.
- **Posições fora do tabuleiro:** `tile(index)` do JS devolve `null` para índices inválidos (inclusive negativos). Em Rust, usar `Option` e contas de vizinhança com inteiros com sinal.
- **Pontos exatos:** os testes esperam valores como 195 e 625. O multiplicador de combo é decimal (+0,4 por cadeia, máximo 3×) e o arredondamento é "para o múltiplo de 5 mais próximo". Conferir que o Rust dá o mesmo valor.
- **IDs das pedras:** `next_id` sobe a cada pedra criada, inclusive as novas que caem. Precisa ser igual ao JS para a etapa 6.
- **Mesma sequência de números aleatórios:** a ordem e a quantidade de sorteios (tabuleiro inicial, pedras novas, Fisher–Yates do embaralhar) devem ser idênticas às do JS, senão a etapa 6 não bate.
- **Cópias do tabuleiro:** o JS guarda uma cópia do tabuleiro em cada evento. Em Rust, clonar.
- **Métodos de apoio que os testes chamam:** `set`, `swap`, `tile`, `gem`, `matches`, `supernovas`, `creations`, `adjacent` precisam ser públicos para os testes portados poderem usá-los.

### O que fica para quem constrói

Divisão exata em módulos e arquivos; tipos numéricos; matriz fixa × `Vec` para o tabuleiro; como fazer o gancho de teste de "sem jogadas"; como montar o gerador da etapa 6. Nomes em inglês (como no original), comentários em português.

## Decisões minhas (o Thiago pode corrigir)

1. Projeto em pasta separada, `prisma-rust/`, como biblioteca com um único pacote; vira "workspace" com mais pacotes só na fase 2.
2. Zero dependências: sem `rand` e sem `serde` agora (JSON entra na fase 2).
3. `restore` recebe uma struct Rust, não texto JSON.
4. Comentários didáticos em português; nomes de código em inglês.
5. A verificação cruzada com o JS (etapa 6) entra como etapa extra.
6. Os cheats e seus testes entram na fase 1 (o painel de desenvolvedor do app usa eles).

## Em aberto

- **Onde guardar o código de forma permanente:** o ideal é um repositório novo no GitHub (por exemplo `prisma-rust`), criado pelo Thiago; só então eu peço acesso de escrita. Não trava as etapas 1 a 3.
