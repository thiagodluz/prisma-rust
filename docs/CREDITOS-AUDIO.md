# Prisma em Rust — créditos do áudio

As músicas e os ambientes do modo Zen são obras de terceiros, as mesmas do Prisma em JS
(<https://github.com/thiagodluz/Prisma>, release v1.4.2). **Oito** delas são de domínio público
([CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/)): os créditos abaixo são uma cortesia.
A **nona**, o "Ruído suave", usa a [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) e **exige
atribuição** (a seção logo depois da tabela).

Os arquivos de áudio **não ficam no git**. O comando `prisma_tools audio game/sons` (pasta `tools/`,
escrito em Rust) baixa a release do Prisma em JS, confere o SHA-256 de cada áudio e os prepara; o
fluxo do Android roda esse comando antes de gerar o APK, e os arquivos vão para dentro dele em
`assets/musica/` e `assets/ambiente/`.

| Arquivo no APK | Obra e autor | Licença | Fonte | O que foi alterado |
| --- | --- | --- | --- | --- |
| `musica/magic-puzzle.ogg` | "Magic Puzzle In-Game 1" — MintoDog | CC0 1.0 | [OpenGameArt](https://opengameart.org/content/magic-puzzle-in-game-1) | nada (o arquivo veio assim) |
| `musica/cozy-puzzle.ogg` | "Cozy Puzzle In-Game 3" — MintoDog | CC0 1.0 | [OpenGameArt](https://opengameart.org/content/cozy-puzzle-in-game-3) | nada (o arquivo veio assim) |
| `musica/space-city.ogg` | "Space City" — MintoDog | CC0 1.0 | [OpenGameArt](https://opengameart.org/content/space-city) | nada (o arquivo veio assim) |
| `ambiente/rain-soft.ogg` | "Soft Rain Loop" — _lynks | CC0 1.0 | [Freesound, som 595717](https://freesound.org/people/_lynks/sounds/595717/) | reconvertido a 44,1 kHz (a gravação já era curta) |
| `ambiente/forest-cicadas.ogg` | "cicadas.wav" — fraska2 | CC0 1.0 | [Freesound, som 434675](https://freesound.org/people/fraska2/sounds/434675/) | reconvertido a 44,1 kHz (a gravação já era curta) |
| `ambiente/rainforest.ogg` | "Rain Sound and Rainforest.mp3" — INNORECORDS | CC0 1.0 | [Freesound, som 457447](https://freesound.org/people/INNORECORDS/sounds/457447/) | cortado em laço de 60 s e reconvertido a 44,1 kHz |
| `ambiente/stream.ogg` | "creek loop" — soundofsong | CC0 1.0 | [Freesound, som 841987](https://freesound.org/people/soundofsong/sounds/841987/) | cortado em laço de 60 s e reconvertido a 44,1 kHz |
| `ambiente/rain-thunder.ogg` | "distant rain and thunder" — bastipictures | CC0 1.0 | [Freesound, som 243777](https://freesound.org/people/bastipictures/sounds/243777/) | cortado em laço de 60 s e reconvertido a 44,1 kHz |
| `ambiente/soft-noise.ogg` | "Gentle White Noise Air Tone" — Geoff Bremner | CC BY 4.0 | [Freesound, som 725423](https://freesound.org/people/Geoff-Bremner-Audio/sounds/725423/) | cortado em laço de 60 s e reconvertido a 44,1 kHz |

## Ruído suave — CC BY 4.0

`ambiente/soft-noise.ogg`: ["Gentle White Noise Air Tone"](https://freesound.org/people/Geoff-Bremner-Audio/sounds/725423/)
por [Geoff Bremner](https://freesound.org/people/Geoff-Bremner-Audio/), sob a
[Creative Commons Atribuição 4.0 Internacional (CC BY 4.0)](https://creativecommons.org/licenses/by/4.0/).

**Alterações:** a gravação foi cortada num laço de 60 s (um trecho do meio, com 3 s de transição suave
entre o fim e o começo, para repetir sem estalo) e reconvertida a 44,1 kHz em Ogg Vorbis, pelo
`prisma_tools audio`. No jogo ela toca em repetição. Quem fez a gravação não aprovou nem apoia o Prisma.

## O que "alterado" quer dizer nos outros ambientes

O Macroquad (a biblioteca do jogo) descompacta o áudio inteiro na memória ao carregá-lo (uns
21 MB por minuto). Por isso as gravações longas viram laços de 60 s e todas passam a 44,1 kHz. As
três músicas ficam exatamente como vieram.

## Efeitos sonoros

Os efeitos (troca inválida, combinação, cascata, Pulso, Raio, Espectro, criação, nível e
recuperação) não são gravações de ninguém: o código do jogo os sintetiza (`game/src/efeitos.rs`),
portanto seguem a licença do código (GPL-3.0-only).

## Créditos dentro do app

A tela de créditos dentro do app ainda não existe: ela vem no marco M4 (polimento). Até lá, esta
página é o lugar dos créditos.
