//! Cheats de desenvolvedor.
//!
//! Todo cheat **válido** guarda o estado para o "desfazer" e marca `cheats_used = true`.
//! Um pedido inválido devolve `false` (ou uma jogada inválida) e não muda absolutamente nada.

use crate::event::MoveResult;
use crate::game::Game;
use crate::matching::{Matches, Run};
use crate::progression::{level_goal, level_start_at};
use crate::tile::{COLORS, SIZE, Tile, TileKind, position};

impl Game {
    /// Soma pontos (de 1 a 1.000.000 por vez; o total nunca passa de 1.000.000.000).
    /// Sobe de nível se a meta for alcançada.
    pub fn cheat_add_points(&mut self, amount: i64) -> bool {
        // `try_from` converte o número só se couber no tipo novo (aqui, sem sinal).
        let Ok(amount) = u64::try_from(amount) else {
            return false;
        };
        if !(1..=1_000_000).contains(&amount) {
            return false;
        }
        match self.score.checked_add(amount) {
            Some(total) if total <= 1_000_000_000 => {}
            _ => return false,
        }
        self.remember_state(self.snapshot());
        self.cheats_used = true;
        self.score += amount;
        while self.score.saturating_sub(self.level_start_score) >= level_goal(self.level) {
            self.level_start_score += level_goal(self.level);
            self.level = self.level.saturating_add(1);
        }
        true
    }

    /// Vai direto para o nível `level` (de 1 a 10.000), mantendo o progresso dentro do nível.
    pub fn cheat_set_level(&mut self, level: i64) -> bool {
        let Some(level) = u32::try_from(level).ok().filter(|level| (1..=10_000).contains(level)) else {
            return false;
        };
        let Some(start) = level_start_at(u64::from(level), 6000) else {
            return false;
        };
        self.remember_state(self.snapshot());
        self.cheats_used = true;
        // O progresso atual cabe no novo nível (nunca passa da meta dele, menos 1).
        let progress = self
            .score
            .saturating_sub(self.level_start_score)
            .min(level_goal(level) - 1);
        self.level = level;
        self.level_start_score = start;
        self.progression_offset = 0;
        self.score = start + progress;
        true
    }

    /// Troca a pedra da casa `index` (0 a 63) por uma pedra especial nova do tipo `kind`.
    ///
    /// Uma pedra comum não vale como "especial". Um Pulso ou Raio no lugar de um Espectro
    /// (que não tem cor) escolhe a primeira cor que não forma combinação.
    pub fn cheat_spawn_special(&mut self, index: i64, kind: TileKind) -> bool {
        let Some(index) = usize::try_from(index).ok().filter(|&index| index < SIZE * SIZE) else {
            return false;
        };
        if kind == TileKind::Normal {
            return false;
        }
        let previous = self.snapshot();
        let Some(old) = self.tile(index) else {
            return false;
        };
        let mut color = old.color;
        if kind != TileKind::Spectrum && color.is_none() {
            // Testa as cores uma a uma no próprio tabuleiro e depois desfaz o teste.
            color = (0..COLORS).find(|&candidate| {
                self.set(index, Tile { color: Some(candidate), kind, ..old });
                self.matches().cells.is_empty()
            });
            self.set(index, old);
            if color.is_none() {
                return false;
            }
        }
        self.remember_state(previous);
        self.cheats_used = true;
        let tile = self.gem(color, kind);
        self.set(index, tile);
        true
    }

    /// Embaralha o tabuleiro, mesmo no Clássico e mesmo com a partida encerrada (que é retomada).
    pub fn cheat_shuffle(&mut self) -> bool {
        let previous = self.snapshot();
        self.ended = false; // o embaralhar automático não funciona numa partida encerrada
        if !self.shuffle(true) {
            self.ended = previous.ended;
            return false;
        }
        self.remember_state(previous);
        self.cheats_used = true;
        true
    }

    /// Liga ou desliga o modo "sem derrota". Devolve `false` se já estava assim.
    /// Ao ligar, uma partida encerrada é retomada e, sem jogadas, ganha um Espectro.
    pub fn cheat_no_defeat(&mut self, enabled: bool) -> bool {
        if enabled == self.no_defeat {
            return false;
        }
        self.remember_state(self.snapshot());
        self.cheats_used = true;
        self.no_defeat = enabled;
        if enabled {
            self.ended = false;
            if !self.has_move() {
                self.rescue();
            }
        }
        true
    }

    /// Explode o tabuleiro inteiro: limpa as 64 casas, dispara cada pedra especial uma vez
    /// e deixa a reação em cadeia seguir.
    pub fn cheat_explode(&mut self) -> MoveResult {
        self.remember_state(self.snapshot());
        self.cheats_used = true;
        self.ended = false;
        let everything: Vec<usize> = (0..SIZE * SIZE).collect();
        let nothing = Matches { cells: Vec::new(), runs: Vec::new() };
        self.resolve_board(nothing, None, &[], &everything)
    }

    /// Força uma Supernova perto da casa `index`: monta Pulso, Raio, Pulso lado a lado
    /// (com a cor da casa) e deixa a fusão real acontecer. Índice inválido = jogada inválida.
    pub fn cheat_supernova(&mut self, index: i64) -> MoveResult {
        let Some(index) = usize::try_from(index).ok().filter(|&index| index < SIZE * SIZE) else {
            return MoveResult::invalid();
        };
        self.remember_state(self.snapshot());
        self.cheats_used = true;
        self.ended = false;
        let (row, column) = position(index);
        // A trinca precisa caber na linha: o centro nunca fica na primeira nem na última coluna.
        let center = row * SIZE + column.clamp(1, SIZE - 2);
        let sources = vec![center - 1, center, center + 1];
        let color = self.tile(index).and_then(|tile| tile.color).unwrap_or(0);
        for (offset, &cell) in sources.iter().enumerate() {
            let kind = if offset == 1 { TileKind::Cross } else { TileKind::Burst };
            let tile = self.gem(Some(color), kind);
            self.set(cell, tile);
        }
        let run = Run { cells: sources.clone(), color, length: 3 };
        let matches = Matches { cells: sources, runs: vec![run] };
        self.resolve_board(matches, None, &[], &[])
    }
}
