//! Toque e arrasto no tabuleiro.
//!
//! Este módulo transforma o que o dedo (ou o mouse) faz em "troque a casa A com a casa B".
//! Não desenha nada e não sabe nada sobre as regras do jogo: só lê gestos. É um módulo puro
//! (só números), por isso dá para testar sem janela. As regras são as do `app.js` do Prisma
//! em JS: tocar numa casa a marca; tocar numa vizinha troca; arrastar para a vizinha troca.

use prisma_engine::{adjacent, SIZE};

use crate::layout::Rect;

/// Quanto o dedo pode andar (em fração da largura da casa) e ainda contar como um "toque".
/// No JS são 12 pixels para casas de uns 46 px; usamos fração porque os celulares variam.
const LIMITE_TOQUE: f32 = 0.25;
/// Quanto o dedo precisa andar (em fração da casa) para contar como um "arrasto".
/// Entre os dois limites o gesto não vale nada (o dedo escorregou).
const LIMITE_ARRASTO: f32 = 0.28;

/// Algo que o ponteiro (dedo ou mouse) fez. As posições são em pixels da tela.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Evento {
    /// O dedo encostou na tela.
    Baixo { x: f32, y: f32 },
    /// O dedo andou, ainda encostado.
    Mover { x: f32, y: f32 },
    /// O dedo saiu da tela.
    Cima { x: f32, y: f32 },
    /// O gesto foi interrompido (por exemplo, o sistema tomou conta da tela).
    Cancelar,
}

/// O que o gesto pede que o jogo faça.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Acao {
    Nada,
    /// Trocar de lugar as pedras destas duas casas (que são vizinhas).
    Trocar(usize, usize),
}

/// Em que casa (0 a 63) do tabuleiro cai o ponto (`x`, `y`)? `None` se cair fora.
///
/// `Option` é "um valor OU nada": aqui, "o número da casa" ou "nenhuma casa".
pub fn casa_em(tabuleiro: Rect, x: f32, y: f32) -> Option<usize> {
    // `contem` já recusa pontos fora, tabuleiro sem tamanho e números inválidos (NaN).
    if !tabuleiro.contem(x, y) {
        return None;
    }
    let largura_da_casa = tabuleiro.w / SIZE as f32;
    let altura_da_casa = tabuleiro.h / SIZE as f32;
    // `as usize` corta a parte depois da vírgula. O `.min(SIZE - 1)` é um cinto de segurança
    // contra arredondamento de número com vírgula bem na borda de baixo/direita.
    let coluna = (((x - tabuleiro.x) / largura_da_casa) as usize).min(SIZE - 1);
    let linha = (((y - tabuleiro.y) / altura_da_casa) as usize).min(SIZE - 1);
    Some(linha * SIZE + coluna)
}

/// Onde o dedo começou a apertar.
#[derive(Clone, Copy)]
struct Inicio {
    casa: usize,
    x: f32,
    y: f32,
}

/// O que se sabe sobre o gesto em andamento e sobre a casa marcada por um toque.
///
/// `#[derive(Default)]` cria o valor "vazio": nada marcado e nenhum gesto em andamento.
#[derive(Default)]
pub struct Gestos {
    selecionada: Option<usize>,
    inicio: Option<Inicio>,
    alvo: Option<usize>,
}

impl Gestos {
    /// A casa marcada por um toque (a tela a destaca).
    pub fn selecionada(&self) -> Option<usize> {
        self.selecionada
    }

    /// A casa que está sob o dedo agora.
    pub fn pressionada(&self) -> Option<usize> {
        self.inicio.map(|inicio| inicio.casa)
    }

    /// A vizinha que o arrasto atual trocaria, se o dedo fosse solto agora.
    pub fn alvo_do_arrasto(&self) -> Option<usize> {
        self.alvo
    }

    /// Esquece a marcação e o gesto (por exemplo, quando começa uma animação ou um jogo novo).
    pub fn limpar(&mut self) {
        *self = Gestos::default();
    }

    /// Trata um evento do ponteiro.
    ///
    /// `tabuleiro` é onde o tabuleiro está na tela agora. `bloqueado` vale `true` enquanto o
    /// jogo não aceita jogadas (animação, diálogo, fim de partida): nesse caso o dedo é ignorado.
    pub fn tratar(&mut self, evento: Evento, tabuleiro: Rect, bloqueado: bool) -> Acao {
        let lado = tabuleiro.w / SIZE as f32;
        match evento {
            Evento::Baixo { x, y } => {
                // Segundo dedo (já há um gesto) e toque fora do tabuleiro não contam.
                if bloqueado || self.inicio.is_some() {
                    return Acao::Nada;
                }
                if let Some(casa) = casa_em(tabuleiro, x, y) {
                    self.inicio = Some(Inicio { casa, x, y });
                    self.alvo = None;
                }
                Acao::Nada
            }
            Evento::Mover { x, y } => {
                if let Some(inicio) = self.inicio {
                    self.alvo = alvo_do_gesto(inicio, x, y, lado);
                }
                Acao::Nada
            }
            Evento::Cima { x, y } => {
                // `take()` tira o gesto do lugar (deixa `None`): soltar sempre encerra o gesto.
                let Some(inicio) = self.inicio.take() else {
                    return Acao::Nada;
                };
                self.alvo = None;
                if bloqueado {
                    return Acao::Nada;
                }
                if let Some(fim) = alvo_do_gesto(inicio, x, y, lado) {
                    self.selecionada = None;
                    return Acao::Trocar(inicio.casa, fim);
                }
                let andou = (x - inicio.x).abs().max((y - inicio.y).abs());
                if andou < LIMITE_TOQUE * lado {
                    return self.tocar_na_casa(inicio.casa);
                }
                Acao::Nada
            }
            Evento::Cancelar => {
                self.inicio = None;
                self.alvo = None;
                Acao::Nada
            }
        }
    }

    /// Um toque na casa: marca, desmarca, troca com a vizinha ou muda a marcação.
    fn tocar_na_casa(&mut self, casa: usize) -> Acao {
        match self.selecionada {
            None => {
                self.selecionada = Some(casa);
                Acao::Nada
            }
            Some(marcada) if marcada == casa => {
                self.selecionada = None;
                Acao::Nada
            }
            Some(marcada) if adjacent(marcada, casa) => {
                self.selecionada = None;
                Acao::Trocar(marcada, casa)
            }
            Some(_) => {
                self.selecionada = Some(casa);
                Acao::Nada
            }
        }
    }
}

/// A casa vizinha para onde um arrasto (de `inicio` até o ponto `x`, `y`) aponta, se houver.
fn alvo_do_gesto(inicio: Inicio, x: f32, y: f32, lado: f32) -> Option<usize> {
    let dx = x - inicio.x;
    let dy = y - inicio.y;
    // Escrito assim (e não com `<`) para um número inválido (NaN) cair em "não é arrasto".
    if !(dx.abs().max(dy.abs()) >= LIMITE_ARRASTO * lado) {
        return None;
    }
    // Um passo só, no eixo em que o dedo mais andou (empate: vertical, como no JS).
    let passo: isize = if dx.abs() > dy.abs() {
        if dx > 0.0 { 1 } else { -1 }
    } else if dy > 0.0 {
        SIZE as isize
    } else {
        -(SIZE as isize)
    };
    let fim = inicio.casa as isize + passo;
    if fim < 0 {
        return None;
    }
    let fim = fim as usize;
    // `adjacent` recusa a "volta" de linha (7 → 8) e o que sai do tabuleiro (64 ou mais).
    adjacent(inicio.casa, fim).then_some(fim)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// O tabuleiro de teste: começa em (100, 200) e tem 400 px de lado, então cada casa mede 50 px.
    const TAB: Rect = Rect { x: 100.0, y: 200.0, w: 400.0, h: 400.0 };

    /// O centro de uma casa (0 a 63) no tabuleiro de teste.
    fn centro(casa: usize) -> (f32, f32) {
        let (coluna, linha) = ((casa % 8) as f32, (casa / 8) as f32);
        (TAB.x + (coluna + 0.5) * 50.0, TAB.y + (linha + 0.5) * 50.0)
    }

    /// Um toque: apertar e soltar no mesmo ponto, no centro da casa.
    fn toque(gestos: &mut Gestos, casa: usize) -> Acao {
        let (x, y) = centro(casa);
        gestos.tratar(Evento::Baixo { x, y }, TAB, false);
        gestos.tratar(Evento::Cima { x, y }, TAB, false)
    }

    /// Um arrasto: apertar no centro da casa, mover `dx`/`dy` pixels e soltar. Devolve o alvo
    /// que o arrasto tinha enquanto o dedo estava em cima, e a ação na hora de soltar.
    fn arrasto(gestos: &mut Gestos, casa: usize, dx: f32, dy: f32) -> (Option<usize>, Acao) {
        let (x, y) = centro(casa);
        gestos.tratar(Evento::Baixo { x, y }, TAB, false);
        gestos.tratar(Evento::Mover { x: x + dx, y: y + dy }, TAB, false);
        let alvo = gestos.alvo_do_arrasto();
        let acao = gestos.tratar(Evento::Cima { x: x + dx, y: y + dy }, TAB, false);
        (alvo, acao)
    }

    #[test]
    fn toque_seleciona_depois_troca_com_a_vizinha() {
        let mut gestos = Gestos::default();
        assert_eq!(toque(&mut gestos, 27), Acao::Nada);
        assert_eq!(gestos.selecionada(), Some(27));
        assert_eq!(toque(&mut gestos, 28), Acao::Trocar(27, 28));
        assert_eq!(gestos.selecionada(), None);
    }

    #[test]
    fn toque_na_mesma_casa_desmarca() {
        let mut gestos = Gestos::default();
        toque(&mut gestos, 27);
        assert_eq!(toque(&mut gestos, 27), Acao::Nada);
        assert_eq!(gestos.selecionada(), None);
    }

    #[test]
    fn toque_em_casa_longe_passa_a_selecao() {
        let mut gestos = Gestos::default();
        toque(&mut gestos, 27);
        // A 30 não é vizinha da 27: a seleção só muda de lugar, sem trocar nada.
        assert_eq!(toque(&mut gestos, 30), Acao::Nada);
        assert_eq!(gestos.selecionada(), Some(30));
        // Diagonal também não é vizinha.
        assert_eq!(toque(&mut gestos, 37), Acao::Nada);
        assert_eq!(gestos.selecionada(), Some(37));
    }

    #[test]
    fn arrasto_para_a_direita_troca_com_a_casa_ao_lado() {
        // Casa 26 = linha 3, coluna 2. Vizinhas: 27 (direita), 25 (esquerda), 18 (cima), 34 (baixo).
        for (dx, dy, vizinha) in [(30.0, 0.0, 27), (-30.0, 0.0, 25), (0.0, -30.0, 18), (0.0, 30.0, 34)] {
            let mut gestos = Gestos::default();
            let (alvo, acao) = arrasto(&mut gestos, 26, dx, dy);
            assert_eq!(alvo, Some(vizinha), "alvo do arrasto ({dx}, {dy})");
            assert_eq!(acao, Acao::Trocar(26, vizinha), "ação ao soltar ({dx}, {dy})");
            assert_eq!(gestos.alvo_do_arrasto(), None, "soltar encerra o gesto");
            assert_eq!(gestos.pressionada(), None);
        }
    }

    #[test]
    fn arrasto_nas_bordas_nao_da_a_volta() {
        // Casa 7 (última coluna) para a direita seria a 8, que é o começo da linha de baixo.
        // Casa 8 (primeira coluna) para a esquerda seria a 7. Casa 3 (primeira linha) para cima
        // e casa 59 (última linha) para baixo saem do tabuleiro.
        for (casa, dx, dy) in [(7, 30.0, 0.0), (8, -30.0, 0.0), (3, 0.0, -30.0), (59, 0.0, 30.0)] {
            let mut gestos = Gestos::default();
            let (alvo, acao) = arrasto(&mut gestos, casa, dx, dy);
            assert_eq!(alvo, None, "casa {casa} ({dx}, {dy})");
            assert_eq!(acao, Acao::Nada, "casa {casa} ({dx}, {dy})");
            assert_eq!(gestos.selecionada(), None);
        }
    }

    #[test]
    fn arrasto_diagonal_vale_o_eixo_maior_e_empate_vale_vertical() {
        let mut gestos = Gestos::default();
        assert_eq!(arrasto(&mut gestos, 26, 30.0, 20.0).1, Acao::Trocar(26, 27), "mais para a direita");
        assert_eq!(arrasto(&mut gestos, 26, 20.0, 30.0).1, Acao::Trocar(26, 34), "mais para baixo");
        assert_eq!(arrasto(&mut gestos, 26, 30.0, 30.0).1, Acao::Trocar(26, 34), "empate: vertical, para baixo");
        assert_eq!(arrasto(&mut gestos, 26, -30.0, -30.0).1, Acao::Trocar(26, 18), "empate: vertical, para cima");
    }

    #[test]
    fn limiares_de_toque_e_arrasto() {
        // A casa mede 50 px: toque vale até 0,25 casa (12,5 px); arrasto a partir de 0,28 casa (14 px).
        let mut gestos = Gestos::default();
        // 5 px (0,1 casa): ainda é um toque.
        assert_eq!(arrasto(&mut gestos, 26, 5.0, 0.0), (None, Acao::Nada));
        assert_eq!(gestos.selecionada(), Some(26));
        gestos.limpar();
        // 13 px (0,26 casa): nem toque nem arrasto, então nada acontece.
        assert_eq!(arrasto(&mut gestos, 26, 13.0, 0.0), (None, Acao::Nada));
        assert_eq!(gestos.selecionada(), None);
        // 15 px (0,3 casa): é um arrasto.
        assert_eq!(arrasto(&mut gestos, 26, 15.0, 0.0), (Some(27), Acao::Trocar(26, 27)));
    }

    #[test]
    fn toque_fora_do_tabuleiro_e_ignorado() {
        let mut gestos = Gestos::default();
        assert_eq!(gestos.tratar(Evento::Baixo { x: 10.0, y: 10.0 }, TAB, false), Acao::Nada);
        assert_eq!(gestos.pressionada(), None);
        assert_eq!(gestos.tratar(Evento::Cima { x: 10.0, y: 10.0 }, TAB, false), Acao::Nada);
        assert_eq!(gestos.selecionada(), None);
    }

    #[test]
    fn casa_em_nas_bordas() {
        assert_eq!(casa_em(TAB, 100.0, 200.0), Some(0), "canto de cima à esquerda conta como dentro");
        assert_eq!(casa_em(TAB, 499.9, 599.9), Some(63));
        assert_eq!(casa_em(TAB, 150.0, 200.0), Some(1), "a divisa entre as casas 0 e 1 pertence à 1");
        assert_eq!(casa_em(TAB, 100.0, 250.0), Some(8));
        assert_eq!(casa_em(TAB, 500.0, 300.0), None, "a borda da direita conta como fora");
        assert_eq!(casa_em(TAB, 300.0, 600.0), None, "a borda de baixo conta como fora");
        assert_eq!(casa_em(TAB, 99.9, 300.0), None);
        assert_eq!(casa_em(TAB, 300.0, 199.9), None);
    }

    #[test]
    fn bloqueado_ignora_tudo() {
        let mut gestos = Gestos::default();
        let (x, y) = centro(26);
        // Bloqueado desde o começo: o apertar nem cria gesto.
        assert_eq!(gestos.tratar(Evento::Baixo { x, y }, TAB, true), Acao::Nada);
        assert_eq!(gestos.pressionada(), None);
        // Apertou livre, mas a entrada foi bloqueada antes de soltar: nada acontece e o gesto acaba.
        gestos.tratar(Evento::Baixo { x, y }, TAB, false);
        assert_eq!(gestos.pressionada(), Some(26));
        assert_eq!(gestos.tratar(Evento::Cima { x: x + 30.0, y }, TAB, true), Acao::Nada);
        assert_eq!(gestos.pressionada(), None);
        assert_eq!(gestos.selecionada(), None);
    }

    #[test]
    fn segundo_dedo_e_ignorado() {
        let mut gestos = Gestos::default();
        let (x, y) = centro(10);
        gestos.tratar(Evento::Baixo { x, y }, TAB, false);
        let (x2, y2) = centro(20);
        gestos.tratar(Evento::Baixo { x: x2, y: y2 }, TAB, false);
        assert_eq!(gestos.pressionada(), Some(10), "o primeiro dedo continua mandando");
    }

    #[test]
    fn cancelar_solta_o_gesto_e_guarda_a_selecao() {
        let mut gestos = Gestos::default();
        toque(&mut gestos, 26);
        let (x, y) = centro(40);
        gestos.tratar(Evento::Baixo { x, y }, TAB, false);
        assert_eq!(gestos.tratar(Evento::Cancelar, TAB, false), Acao::Nada);
        assert_eq!(gestos.pressionada(), None);
        assert_eq!(gestos.selecionada(), Some(26));
        // Soltar depois de cancelar não faz nada (não há mais gesto).
        assert_eq!(gestos.tratar(Evento::Cima { x, y }, TAB, false), Acao::Nada);
        assert_eq!(gestos.selecionada(), Some(26));
    }

    #[test]
    fn limpar_zera_selecao_e_gesto() {
        let mut gestos = Gestos::default();
        toque(&mut gestos, 26);
        let (x, y) = centro(40);
        gestos.tratar(Evento::Baixo { x, y }, TAB, false);
        gestos.limpar();
        assert_eq!(gestos.selecionada(), None);
        assert_eq!(gestos.pressionada(), None);
        assert_eq!(gestos.alvo_do_arrasto(), None);
    }

    #[test]
    fn tabuleiro_vazio_ou_invalido_nao_da_pane() {
        let vazio = Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 };
        let com_nan = Rect { x: f32::NAN, y: 0.0, w: 100.0, h: 100.0 };
        for tabuleiro in [vazio, com_nan] {
            assert_eq!(casa_em(tabuleiro, 0.0, 0.0), None);
            assert_eq!(casa_em(tabuleiro, 50.0, 50.0), None);
            let mut gestos = Gestos::default();
            assert_eq!(gestos.tratar(Evento::Baixo { x: 1.0, y: 1.0 }, tabuleiro, false), Acao::Nada);
            assert_eq!(gestos.tratar(Evento::Mover { x: 9.0, y: 9.0 }, tabuleiro, false), Acao::Nada);
            assert_eq!(gestos.tratar(Evento::Cima { x: 9.0, y: 9.0 }, tabuleiro, false), Acao::Nada);
        }
    }

    #[test]
    fn mover_e_soltar_sem_apertar_nao_fazem_nada() {
        let mut gestos = Gestos::default();
        let (x, y) = centro(26);
        assert_eq!(gestos.tratar(Evento::Mover { x, y }, TAB, false), Acao::Nada);
        assert_eq!(gestos.alvo_do_arrasto(), None);
        assert_eq!(gestos.tratar(Evento::Cima { x, y }, TAB, false), Acao::Nada);
        assert_eq!(gestos.selecionada(), None);
    }
}
