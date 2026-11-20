//! A jogada: trocar duas pedras e resolver toda a reação em cadeia (limpar, cair, repetir).

use crate::event::{Activation, Created, Event, MoveResult};
use crate::game::Game;
use crate::matching::{Matches, adjacent};
use crate::progression::level_goal;
use crate::tile::{Mode, SIZE, Tile, TileKind, position};

/// Quando o jogador troca um Espectro de lugar, o efeito dele é direto (não depende de combinação).
pub(crate) struct Direct {
    /// As casas (já depois da troca) onde há Espectro; uma ou duas.
    pub spectrum: Vec<usize>,
    /// A outra casa trocada, quando só um dos dois é Espectro: a cor dela é a que será limpa.
    pub target: Option<usize>,
}

/// Que pedras um Espectro leva embora.
enum Sweep {
    All,
    Color(u8),
    Nothing,
}

/// Pontos de bônus por criar cada tipo de pedra especial.
fn creation_points(kind: TileKind) -> u64 {
    match kind {
        TileKind::Burst => 120,
        TileKind::Cross => 180,
        TileKind::Spectrum => 240,
        TileKind::Normal => 0,
    }
}

/// A casa deslocada `dr` linhas e `dc` colunas de (r, c), ou `None` se cair fora do tabuleiro.
///
/// Usa `i32` (número com sinal) porque o deslocamento pode ser negativo.
fn offset(r: usize, c: usize, dr: i32, dc: i32) -> Option<usize> {
    let row = r as i32 + dr;
    let col = c as i32 + dc;
    let size = SIZE as i32;
    if row < 0 || row >= size || col < 0 || col >= size {
        None
    } else {
        Some(row as usize * SIZE + col as usize)
    }
}

/// O que acontece dentro de UMA limpeza: quais casas saem e quais pedras especiais disparam.
///
/// Ela só olha o jogo (`&Game`): nada muda no tabuleiro até a limpeza terminar.
struct Reaction<'a> {
    game: &'a Game,
    /// Casas onde vai nascer uma pedra especial: não podem ser limpas.
    protected: &'a [usize],
    /// Casas que serão limpas, na ordem em que foram marcadas.
    cleared: Vec<usize>,
    /// Pedras especiais esperando para disparar.
    queue: Vec<usize>,
    /// Ids das pedras que já dispararam (cada uma dispara uma vez só).
    seen: Vec<u32>,
    activated: Vec<Activation>,
}

impl Reaction<'_> {
    /// Põe uma pedra especial na fila para disparar (se ainda não disparou).
    fn enqueue(&mut self, index: usize) {
        if let Some(tile) = self.game.tile(index)
            && !self.protected.contains(&index)
            && tile.kind != TileKind::Normal
            && !self.seen.contains(&tile.id)
        {
            self.queue.push(index);
        }
    }

    /// Marca a casa como limpa (sem repetir) e agenda o disparo se for uma pedra especial.
    fn include(&mut self, index: usize) {
        if !self.cleared.contains(&index) {
            self.cleared.push(index);
        }
        self.enqueue(index);
    }

    /// Como `include`, mas ignora casas fora do tabuleiro e casas protegidas.
    fn add(&mut self, index: usize) {
        if index >= SIZE * SIZE || self.protected.contains(&index) {
            return;
        }
        self.include(index);
    }

    /// Qual cor (ou todas) este Espectro leva embora.
    fn sweep_for(&self, index: usize, direct: Option<&Direct>) -> Sweep {
        let color = match direct {
            Some(d) if d.spectrum.len() == 2 => return Sweep::All,
            Some(d) if d.spectrum.contains(&index) => {
                d.target.and_then(|target| self.game.tile(target)).and_then(|tile| tile.color)
            }
            _ => self.game.nearby_color(index, &self.cleared),
        };
        match color {
            Some(color) => Sweep::Color(color),
            None => Sweep::Nothing,
        }
    }

    /// Dispara as pedras especiais da fila, uma a uma, até acabar (um disparo pode empilhar outros).
    fn run(&mut self, direct: Option<&Direct>) {
        while !self.queue.is_empty() {
            // Pulsos e Raios disparam antes dos Espectros, para o Espectro enxergar o que sobrou.
            let first_regular = self
                .queue
                .iter()
                .position(|&cell| self.game.tile(cell).map(|tile| tile.kind) != Some(TileKind::Spectrum))
                .unwrap_or(0);
            let index = self.queue.remove(first_regular);
            let Some(tile) = self.game.tile(index) else { continue };
            if self.seen.contains(&tile.id) {
                continue;
            }
            self.seen.push(tile.id);
            self.activated.push(Activation::Special { index, kind: tile.kind, id: tile.id });
            let (r, c) = position(index);
            match tile.kind {
                TileKind::Burst => {
                    for dr in -1i32..=1 {
                        for dc in -1i32..=1 {
                            if let Some(cell) = offset(r, c, dr, dc) {
                                self.add(cell);
                            }
                        }
                    }
                }
                TileKind::Cross => {
                    for i in 0..SIZE {
                        self.add(r * SIZE + i);
                        self.add(i * SIZE + c);
                    }
                }
                TileKind::Spectrum => {
                    let sweep = self.sweep_for(index, direct);
                    for i in 0..SIZE * SIZE {
                        let hit = match sweep {
                            Sweep::All => true,
                            Sweep::Color(color) => {
                                self.game.tile(i).and_then(|tile| tile.color) == Some(color)
                            }
                            Sweep::Nothing => false,
                        };
                        if hit {
                            self.add(i);
                        }
                    }
                }
                TileKind::Normal => {}
            }
        }
    }
}

impl Game {
    /// Tenta trocar de lugar as pedras das casas `a` e `b` e resolve tudo o que acontece depois.
    ///
    /// É o `move` do JS (`move` é uma palavra reservada em Rust). Se a troca não for permitida,
    /// nada muda e o resultado tem `valid == false`.
    pub fn play(&mut self, a: usize, b: usize) -> MoveResult {
        if self.ended || !adjacent(a, b) {
            return MoveResult::invalid();
        }
        let previous = self.snapshot(); // o estado de antes da troca, para o "desfazer"
        self.swap(a, b);
        let found = self.matches();
        let spectrum: Vec<usize> = [a, b]
            .into_iter()
            .filter(|&index| self.tile(index).map(|tile| tile.kind) == Some(TileKind::Spectrum))
            .collect();
        if found.cells.is_empty() && spectrum.is_empty() {
            self.swap(a, b); // a troca não serviu: desfaz
            return MoveResult::invalid();
        }
        self.remember_state(previous); // só jogadas válidas entram no histórico
        let direct = if spectrum.is_empty() {
            None
        } else {
            let target = if spectrum.len() == 1 {
                Some(if spectrum[0] == a { b } else { a })
            } else {
                None
            };
            Some(Direct { spectrum, target })
        };
        self.resolve_board(found, direct, &[b, a], &[])
    }

    /// Repete "limpar, cair, procurar novas combinações" até o tabuleiro assentar. Soma os pontos e sobe de nível.
    pub(crate) fn resolve_board(
        &mut self,
        mut matches: Matches,
        mut direct: Option<Direct>,
        preferred: &[usize],
        initial_cells: &[usize],
    ) -> MoveResult {
        let mut initial: Vec<usize> = initial_cells.to_vec();
        let mut events: Vec<Event> = Vec::new();
        let mut chain: u32 = 0;
        let mut earned: u64 = 0;

        while !matches.cells.is_empty() || direct.is_some() || !initial.is_empty() {
            chain += 1;
            let supernovas = self.supernovas(&matches);
            let fused: Vec<usize> =
                supernovas.iter().flat_map(|nova| nova.sources.iter().copied()).collect();

            // Pedras especiais que nascem (só na 1ª etapa vale a casa preferida do jogador).
            let creations: Vec<Created> = if direct.is_some() {
                Vec::new()
            } else {
                let wanted: &[usize] = if chain == 1 { preferred } else { &[] };
                self.creations(&matches, wanted, &fused)
                    .into_iter()
                    .map(|creation| Created {
                        index: creation.index,
                        kind: creation.kind,
                        color: creation.color,
                        tile: self.gem(Some(creation.color), creation.kind),
                    })
                    .collect()
            };
            let protected: Vec<usize> = creations.iter().map(|creation| creation.index).collect();
            let seen: Vec<u32> =
                fused.iter().filter_map(|&index| self.tile(index)).map(|tile| tile.id).collect();

            // Descobre tudo o que sai nesta etapa.
            let (cleared, activated) = {
                let mut reaction = Reaction {
                    game: self,
                    protected: &protected,
                    cleared: Vec::new(),
                    queue: Vec::new(),
                    seen,
                    activated: supernovas.iter().cloned().map(Activation::Supernova).collect(),
                };
                for &index in matches.cells.iter().chain(initial.iter()) {
                    if !protected.contains(&index) && !reaction.cleared.contains(&index) {
                        reaction.cleared.push(index);
                    }
                }
                for index in reaction.cleared.clone() {
                    reaction.enqueue(index);
                }
                if let Some(direct) = &direct {
                    for &index in &direct.spectrum {
                        reaction.include(index);
                    }
                    if let Some(target) = direct.target {
                        reaction.include(target);
                    }
                }
                for nova in &supernovas {
                    let (r, c) = position(nova.index);
                    for dr in -2i32..=2 {
                        for dc in -2i32..=2 {
                            if let Some(cell) = offset(r, c, dr, dc) {
                                reaction.add(cell);
                            }
                        }
                    }
                }
                reaction.run(direct.as_ref());
                (reaction.cleared, reaction.activated)
            };

            // Pontos desta etapa.
            let base = cleared.len() as u64 * 25;
            let creation_bonus: u64 =
                creations.iter().map(|creation| creation_points(creation.kind)).sum();
            let combo_multiplier = 1.0 + f64::from((chain - 1).min(5)) * 0.4;
            let gained = ((base + creation_bonus) as f64 * combo_multiplier / 5.0).round() as u64 * 5;
            events.push(Event::Clear {
                board: self.board, // o tabuleiro ainda não mudou: é o "antes" da limpeza
                cells: cleared.clone(),
                activated,
                creations: creations.clone(),
                chain,
                base,
                creation_bonus,
                combo_multiplier,
                earned: gained,
            });
            earned += gained;

            // Remove as pedras limpas e coloca as especiais que nasceram.
            let mut grid: [[Option<Tile>; SIZE]; SIZE] = self.board.map(|row| row.map(Some));
            for &index in &cleared {
                let (r, c) = position(index);
                grid[r][c] = None;
            }
            for creation in &creations {
                let (r, c) = position(creation.index);
                grid[r][c] = Some(creation.tile);
            }

            // Queda: em cada coluna, as pedras que sobram descem e entram pedras novas por cima.
            let mut falls = vec![0i32; SIZE * SIZE];
            for c in 0..SIZE {
                let mut remaining: Vec<(Tile, usize)> = Vec::new(); // (pedra, linha de origem), de baixo para cima
                for r in (0..SIZE).rev() {
                    if let Some(tile) = grid[r][c] {
                        remaining.push((tile, r));
                    }
                }
                let missing = SIZE - remaining.len();
                for r in (0..SIZE).rev() {
                    let existing = remaining.get(SIZE - 1 - r).copied();
                    let (tile, origin_row) = match existing {
                        Some((tile, row)) => (tile, row as i32),
                        None => (self.random_gem(), r as i32 - missing as i32),
                    };
                    self.board[r][c] = tile;
                    // Origem negativa = pedra nova que veio de cima do tabuleiro.
                    falls[r * SIZE + c] = origin_row * SIZE as i32 + c as i32;
                }
            }
            events.push(Event::Fall { board: self.board, falls, chain });

            direct = None;
            initial.clear();
            matches = self.matches();
        }

        // Soma os pontos e sobe de nível enquanto a meta for alcançada.
        self.score += earned;
        let old_level = self.level;
        while self.score - self.level_start_score >= level_goal(self.level) {
            self.level_start_score += level_goal(self.level);
            self.level += 1;
        }

        // Subiu de nível: embaralha uma vez (se não conseguir, segue com o tabuleiro como está).
        let mut shuffled = false;
        if self.level > old_level {
            let before = self.board;
            shuffled = self.shuffle(true);
            if shuffled {
                events.push(Event::Shuffle { before, board: self.board, chain });
            }
        }

        // Sem jogadas: no Clássico a partida acaba; no Zen o jogo cria um Espectro para destravar.
        let mut rescued = false;
        if !self.has_move() {
            if self.mode == Mode::Classic && !self.no_defeat {
                self.ended = true;
                events.push(Event::GameOver { board: self.board, chain });
            } else {
                let index = self.rescue();
                events.push(Event::Rescue { board: self.board, index, chain });
                rescued = true;
            }
        }

        MoveResult {
            valid: true,
            events,
            earned,
            chain,
            rescued,
            shuffled,
            ended: self.ended,
            levels_gained: self.level - old_level,
        }
    }

    /// A cor da pedra mais próxima (na mesma linha, depois no tabuleiro todo) que não foi limpa.
    /// O Espectro que dispara "sem querer" usa essa cor.
    fn nearby_color(&self, index: usize, cleared: &[usize]) -> Option<u8> {
        let (r, c) = position(index);
        let here = (r * SIZE + c) as isize;
        for distance in 1..SIZE as isize {
            for next in [here - distance, here + distance] {
                if next < 0 {
                    continue;
                }
                let next = next as usize;
                // Só vale ficar na mesma linha (sem "dar a volta" para a linha vizinha).
                if next / SIZE != r || cleared.contains(&next) {
                    continue;
                }
                if let Some(color) = self.tile(next).and_then(|tile| tile.color) {
                    return Some(color);
                }
            }
        }
        // A linha toda foi varrida: usa a pedra colorida sobrevivente mais próxima.
        let mut closest: Option<(u8, usize)> = None;
        for cell in 0..SIZE * SIZE {
            if cleared.contains(&cell) {
                continue;
            }
            let Some(color) = self.tile(cell).and_then(|tile| tile.color) else { continue };
            let (row, column) = position(cell);
            let distance = row.abs_diff(r) + column.abs_diff(c);
            if closest.is_none_or(|(_, best)| distance < best) {
                closest = Some((color, distance));
            }
        }
        closest.map(|(color, _)| color)
    }
}
