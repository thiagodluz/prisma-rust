//! As partículas da limpeza: faíscas, onda, feixes, véu, a explosão da Supernova e a fusão que
//! vem antes dela. É um porte do `animateClear` do Prisma em JS, só com números (nada é
//! desenhado aqui), por isso dá para testar sem janela. O desenho está em `render.rs`.
//!
//! Tudo é medido em CASAS a partir do canto de cima à esquerda do tabuleiro: o tabuleiro vai de
//! 0 a 8 nos dois sentidos, e quem desenha multiplica pelo lado de uma casa. O tempo `t_ms` conta
//! do começo do passo de limpeza, já com a fusão (que vem primeiro, só na Supernova).

use crate::linha::{
    fusao_dos_visuais, TipoVisual, Visual, FAISCA_MS, FEIXE_MS, FUSAO_MS, ONDA_MS, SUPERNOVA_MS, VEU_MS,
};
use crate::preferencias::Intensidade;
use prisma_engine::{position, SIZE};

/// O máximo de partículas de uma vez (o pior caso real fica bem abaixo; o teto é uma trava).
pub const MAXIMO_DE_PARTICULAS: usize = 96;

/// Como a partícula é desenhada.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Forma {
    /// Um quadradinho brilhante centrado em (`x`, `y`), com `tamanho` de lado.
    Faisca,
    /// Um anel centrado em (`x`, `y`), com `tamanho` de diâmetro e `espessura` de borda.
    Anel,
    /// Uma linha que sai de (`x`, `y`), mede `tamanho` na direção `angulo` e tem `espessura`.
    Linha,
    /// O tabuleiro todo (de 0 a `tamanho`), numa cor só.
    Veu,
}

/// Uma partícula num instante. `alfa` é a opacidade (0 a 1) e `cor` é RGB (0 a 1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Particula {
    pub forma: Forma,
    pub x: f32,
    pub y: f32,
    pub tamanho: f32,
    pub espessura: f32,
    /// Em radianos. Nas faíscas, a direção em que ela anda; nas linhas, a direção da linha.
    pub angulo: f32,
    pub alfa: f32,
    pub cor: [f32; 3],
}

// ---- Medidas e cores do JS (os pixels viram casas: uma casa tem uns 55 px) -----------------

const LADO_DO_TABULEIRO: f32 = SIZE as f32;
/// A faísca tem 6 px.
const FAISCA_LADO: f32 = 0.11;
/// A borda da onda tem 2 px; a do anel da Supernova, 3 px.
const ONDA_BORDA: f32 = 0.04;
const SUPERNOVA_BORDA: f32 = 0.055;
/// Os feixes do Raio têm 8 px; a linha da fusão, 4 px.
const FEIXE_ESPESSURA: f32 = 0.2;
const FUSAO_ESPESSURA: f32 = 0.08;
/// A onda mede 1,15 casa de diâmetro (antes de crescer); o anel da Supernova, 5.
const ONDA_DIAMETRO: f32 = 1.15;
const SUPERNOVA_DIAMETRO: f32 = 5.0;
/// O centro do véu do Espectro tem 40% de opacidade (`#f3d2ff66` no JS).
const VEU_OPACIDADE: f32 = 0.4;

const DOURADO: [f32; 3] = [1.0, 246.0 / 255.0, 209.0 / 255.0]; // #fff6d1
const AZUL_GELO: [f32; 3] = [181.0 / 255.0, 250.0 / 255.0, 1.0]; // #b5faff
const ONDA_COR: [f32; 3] = [1.0, 233.0 / 255.0, 172.0 / 255.0]; // #ffe9ac
const FEIXE_COR: [f32; 3] = [234.0 / 255.0, 252.0 / 255.0, 1.0]; // #eafcff
const VEU_COR: [f32; 3] = [243.0 / 255.0, 210.0 / 255.0, 1.0]; // #f3d2ff

/// As cores da Supernova, uma por cor de pedra (`novaColors` do JS).
const NOVA_CORES: [u32; 7] = [0xff5774, 0xffad48, 0xffe36b, 0x69ef98, 0x6deeff, 0x6d9fff, 0xd795ff];

/// Uma cor `0xRRGGBB` em três números de 0 a 1.
fn rgb(cor: u32) -> [f32; 3] {
    [((cor >> 16) & 0xff) as f32 / 255.0, ((cor >> 8) & 0xff) as f32 / 255.0, (cor & 0xff) as f32 / 255.0]
}

/// A cor da Supernova (cor de pedra fora da lista usa a última, sem pânico).
fn cor_da_nova(cor: u8) -> [f32; 3] {
    rgb(NOVA_CORES[usize::from(cor).min(NOVA_CORES.len() - 1)])
}

/// `hsl(h 100% 77%)` em RGB (h em graus): as faíscas do Espectro.
fn hsl_claro(h: f32) -> [f32; 3] {
    let (s, l) = (1.0_f32, 0.77_f32);
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [r + m, g + m, b + m]
}

// ---- Curvas --------------------------------------------------------------------------------

/// Que parte da animação já passou (0 a 1), dado o tempo `u` desde o começo dela.
fn fracao(u: f32, duracao: f32) -> f32 {
    if duracao <= 0.0 {
        1.0
    } else {
        (u / duracao).clamp(0.0, 1.0)
    }
}

/// Começa rápido e termina devagar (o `ease-out` do JS).
fn saida(p: f32) -> f32 {
    1.0 - (1.0 - p).powi(3)
}

/// Começa devagar e termina rápido (o `ease-in` do JS).
fn entrada(p: f32) -> f32 {
    p * p
}

/// O valor de uma animação por quadros-chave, como o `animate` do JS: `quadros` são pares
/// (posição de 0 a 1, valor), em ordem, e o valor entre dois quadros é uma média entre eles.
fn quadros(p: f32, quadros: &[(f32, f32)]) -> f32 {
    let p = p.clamp(0.0, 1.0);
    for par in quadros.windows(2) {
        let ((p0, v0), (p1, v1)) = (par[0], par[1]);
        if p <= p1 {
            return if p1 > p0 { v0 + (v1 - v0) * (p - p0) / (p1 - p0) } else { v1 };
        }
    }
    quadros.last().map_or(0.0, |&(_, valor)| valor)
}

/// O meio da casa `casa`, em casas a partir do canto do tabuleiro.
fn centro(casa: usize) -> (f32, f32) {
    let (linha, coluna) = position(casa);
    (coluna as f32 + 0.5, linha as f32 + 0.5)
}

// ---- As partículas ---------------------------------------------------------------------------

/// Quantas faíscas um efeito solta: 5 no Suave; no resto, 12 no Espectro e 8 nos outros.
fn faiscas_do_efeito(tipo: TipoVisual, intensidade: Intensidade) -> usize {
    match (intensidade, tipo) {
        (Intensidade::Suave, _) => 5,
        (_, TipoVisual::Espectro) => 12,
        _ => 8,
    }
}

/// Até onde a faísca chega, em casas.
fn alcance(tipo: Option<TipoVisual>) -> f32 {
    match tipo {
        Some(TipoVisual::Espectro) => 2.2,
        Some(TipoVisual::Raio) => 1.65,
        _ => 1.25,
    }
}

/// `n` faíscas que saem de (`ox`, `oy`) em estrela, `u` ms depois de começarem (a animação toda
/// dura `FAISCA_MS` vezes a escala). O ângulo da `i`-ésima é `2π·i/n + 0,18`.
fn faiscas(ox: f32, oy: f32, tipo: Option<TipoVisual>, n: usize, u: f32, escala_de_tempo: f32, saida_: &mut Vec<Particula>) {
    let p = fracao(u, FAISCA_MS * escala_de_tempo);
    let progresso = saida(p);
    let deslocamento = quadros(progresso, &[(0.0, 0.0), (0.25, 0.32), (1.0, 1.0)]);
    let escala = quadros(progresso, &[(0.0, 0.35), (0.25, 1.15), (1.0, 0.35)]);
    let alfa = quadros(progresso, &[(0.0, 0.0), (0.25, 1.0), (1.0, 0.0)]);
    let raio = alcance(tipo);
    for i in 0..n {
        let angulo = std::f32::consts::TAU * i as f32 / n as f32 + 0.18;
        let cor = match tipo {
            Some(TipoVisual::Raio) => AZUL_GELO,
            Some(TipoVisual::Espectro) => hsl_claro(i as f32 * 360.0 / n as f32),
            _ => DOURADO,
        };
        saida_.push(Particula {
            forma: Forma::Faisca,
            x: ox + raio * angulo.cos() * deslocamento,
            y: oy + raio * angulo.sin() * deslocamento,
            tamanho: FAISCA_LADO * escala,
            espessura: 0.0,
            angulo,
            alfa,
            cor,
        });
    }
}

/// As partículas de um instante da limpeza (vazia depois que tudo acabou). `visuais` são as
/// ativações do passo, `celulas` as pedras que somem (a nuvem de faíscas de uma limpeza sem
/// especial sai da do meio), `t_ms` o tempo desde o começo do passo e `intensidade` a escolha do
/// jogador (que estica ou encolhe os tempos e, no Suave, solta menos faíscas).
///
/// Tempo inválido (NaN ou negativo) vale 0; tempo enorme ou infinito não deixa nada na tela. Nunca
/// passa de `MAXIMO_DE_PARTICULAS`: se passasse, as faíscas são as primeiras a cair fora.
pub fn particulas(visuais: &[Visual], celulas: &[usize], t_ms: f32, intensidade: Intensidade) -> Vec<Particula> {
    let t = if t_ms.is_nan() || t_ms < 0.0 { 0.0 } else { t_ms };
    let escala_de_tempo = intensidade.escala_de_duracao();
    // Só entram casas que existem no tabuleiro.
    let existe = |casa: usize| casa < SIZE * SIZE;
    let visuais: Vec<&Visual> = visuais.iter().filter(|visual| existe(visual.casa)).collect();
    let fusao = fusao_dos_visuais(visuais.iter().copied(), intensidade);
    let mut grandes: Vec<Particula> = Vec::new();
    let mut pequenas: Vec<Particula> = Vec::new();

    if t < fusao {
        // Primeira fase: uma linha de cada pedra de origem até o centro da Supernova.
        let p = fracao(t, FUSAO_MS * escala_de_tempo);
        let progresso = entrada(p);
        let comprimento = quadros(progresso, &[(0.0, 0.0), (0.6, 1.0), (1.0, 1.0)]);
        let alfa = quadros(progresso, &[(0.0, 0.0), (0.6, 1.0), (1.0, 0.6)]);
        for nova in visuais.iter().filter(|visual| visual.tipo == TipoVisual::Supernova) {
            let (cx, cy) = centro(nova.casa);
            for &origem in nova.origens.iter().filter(|&&origem| origem != nova.casa && existe(origem)) {
                let (ox, oy) = centro(origem);
                let (dx, dy) = (cx - ox, cy - oy);
                grandes.push(Particula {
                    forma: Forma::Linha,
                    x: ox,
                    y: oy,
                    tamanho: dx.hypot(dy) * comprimento,
                    espessura: FUSAO_ESPESSURA,
                    angulo: dy.atan2(dx),
                    alfa,
                    cor: cor_da_nova(nova.cor),
                });
            }
        }
    } else {
        // Segunda fase: os efeitos, `u` ms depois do fim da fusão.
        let u = t - fusao;
        // As Supernovas primeiro e depois, no máximo, três especiais, na ordem do motor.
        let novas = visuais.iter().filter(|visual| visual.tipo == TipoVisual::Supernova);
        let outras = visuais.iter().filter(|visual| visual.tipo != TipoVisual::Supernova).take(3);
        for visual in novas.chain(outras) {
            let (x, y) = centro(visual.casa);
            efeito(visual, x, y, u, escala_de_tempo, intensidade, &mut grandes, &mut pequenas);
        }
        // Uma limpeza sem especial: uma nuvem de faíscas saindo da pedra do meio.
        if visuais.is_empty() && !celulas.is_empty() {
            let casa = celulas[celulas.len() / 2];
            if existe(casa) && u < FAISCA_MS * escala_de_tempo {
                let (x, y) = centro(casa);
                let n = if intensidade == Intensidade::Suave { 3 } else { 5 };
                faiscas(x, y, None, n, u, escala_de_tempo, &mut pequenas);
            }
        }
    }

    grandes.extend(pequenas);
    grandes.truncate(MAXIMO_DE_PARTICULAS);
    grandes
}

/// As partículas de um efeito (`visual`) no ponto (`x`, `y`), `u` ms depois de começarem.
#[allow(clippy::too_many_arguments)]
fn efeito(
    visual: &Visual,
    x: f32,
    y: f32,
    u: f32,
    escala_de_tempo: f32,
    intensidade: Intensidade,
    grandes: &mut Vec<Particula>,
    pequenas: &mut Vec<Particula>,
) {
    let tipo = visual.tipo;
    match tipo {
        TipoVisual::Supernova if u < SUPERNOVA_MS * escala_de_tempo => {
            let progresso = saida(fracao(u, SUPERNOVA_MS * escala_de_tempo));
            let escala = quadros(progresso, &[(0.0, 0.08), (0.18, 0.35), (0.65, 1.0), (1.0, 1.0)]);
            let alfa = quadros(progresso, &[(0.0, 0.0), (0.18, 0.9), (0.65, 0.7), (1.0, 0.0)]);
            grandes.push(Particula {
                forma: Forma::Anel,
                x,
                y,
                tamanho: SUPERNOVA_DIAMETRO * escala,
                espessura: SUPERNOVA_BORDA,
                angulo: 0.0,
                alfa,
                cor: cor_da_nova(visual.cor),
            });
        }
        TipoVisual::Pulso | TipoVisual::Raio if u < ONDA_MS * escala_de_tempo => {
            let progresso = saida(fracao(u, ONDA_MS * escala_de_tempo));
            let ultimo = if tipo == TipoVisual::Pulso { 3.0 } else { 2.0 };
            let escala = quadros(progresso, &[(0.0, 0.3), (0.35, 1.2), (1.0, ultimo)]);
            let alfa = quadros(progresso, &[(0.0, 0.0), (0.35, 0.9), (1.0, 0.0)]);
            grandes.push(Particula {
                forma: Forma::Anel,
                x,
                y,
                tamanho: ONDA_DIAMETRO * escala,
                espessura: ONDA_BORDA,
                angulo: 0.0,
                alfa,
                cor: ONDA_COR,
            });
        }
        TipoVisual::Espectro if u < VEU_MS * escala_de_tempo => {
            let progresso = saida(fracao(u, VEU_MS * escala_de_tempo));
            let alfa = quadros(progresso, &[(0.0, 0.0), (0.35, 0.9), (1.0, 0.0)]) * VEU_OPACIDADE;
            grandes.push(Particula {
                forma: Forma::Veu,
                x: 0.0,
                y: 0.0,
                tamanho: LADO_DO_TABULEIRO,
                espessura: 0.0,
                angulo: 0.0,
                alfa,
                cor: VEU_COR,
            });
        }
        _ => {}
    }
    // Os feixes do Raio: uma linha na horizontal e outra na vertical, por todo o tabuleiro, que
    // crescem a partir do meio do tabuleiro.
    if tipo == TipoVisual::Raio && u < FEIXE_MS * escala_de_tempo {
        let progresso = saida(fracao(u, FEIXE_MS * escala_de_tempo));
        let comprimento = LADO_DO_TABULEIRO * quadros(progresso, &[(0.0, 0.0), (0.45, 1.0), (1.0, 1.0)]);
        let alfa = quadros(progresso, &[(0.0, 0.0), (0.45, 1.0), (1.0, 0.0)]);
        let meio = LADO_DO_TABULEIRO / 2.0;
        for (angulo, x_do_feixe, y_do_feixe) in
            [(0.0, meio - comprimento / 2.0, y), (std::f32::consts::FRAC_PI_2, x, meio - comprimento / 2.0)]
        {
            grandes.push(Particula {
                forma: Forma::Linha,
                x: x_do_feixe,
                y: y_do_feixe,
                tamanho: comprimento,
                espessura: FEIXE_ESPESSURA,
                angulo,
                alfa,
                cor: FEIXE_COR,
            });
        }
    }
    // As faíscas: todo efeito solta.
    if u < FAISCA_MS * escala_de_tempo {
        faiscas(x, y, Some(tipo), faiscas_do_efeito(tipo, intensidade), u, escala_de_tempo, pequenas);
    }
}

/// O jeito que o JS dá ao "Efeitos": no Suave a camada de efeitos fica com 48% de opacidade e
/// menos viva (saturação 0,8); no Vibrante, mais viva (saturação 1,35) e mais clara (brilho 1,13).
/// Devolve a cor e o alfa já ajustados; o Padrão não muda nada.
pub fn ajustar(cor: [f32; 3], alfa: f32, intensidade: Intensidade) -> ([f32; 3], f32) {
    let (saturacao, brilho, fator_de_alfa) = match intensidade {
        Intensidade::Suave => (0.8, 1.0, 0.48),
        Intensidade::Padrao => return (cor, alfa),
        Intensidade::Vibrante => (1.35, 1.13, 1.0),
    };
    let luz = 0.2126 * cor[0] + 0.7152 * cor[1] + 0.0722 * cor[2];
    let canal = |valor: f32| ((luz + (valor - luz) * saturacao) * brilho).clamp(0.0, 1.0);
    ([canal(cor[0]), canal(cor[1]), canal(cor[2])], alfa * fator_de_alfa)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PADRAO: Intensidade = Intensidade::Padrao;

    fn visual(casa: usize, tipo: TipoVisual) -> Visual {
        Visual { casa, tipo, cor: 0, origens: vec![] }
    }

    /// Uma Supernova com fusão, de cor 1, com as pedras de origem ao lado da `casa`.
    fn supernova(casa: usize) -> Visual {
        Visual { casa, tipo: TipoVisual::Supernova, cor: 1, origens: vec![casa - 1, casa, casa + 1] }
    }

    fn so(particulas: &[Particula], forma: Forma) -> Vec<Particula> {
        particulas.iter().copied().filter(|particula| particula.forma == forma).collect()
    }

    fn numeros(particula: &Particula) -> Vec<f32> {
        let mut todos = vec![particula.x, particula.y, particula.tamanho, particula.espessura, particula.angulo, particula.alfa];
        todos.extend(particula.cor);
        todos
    }

    #[test]
    fn limpeza_comum_solta_5_faiscas_no_padrao_e_3_no_suave() {
        let celulas = [10, 11, 12, 13, 14];
        for (intensidade, quantas) in [(PADRAO, 5), (Intensidade::Suave, 3), (Intensidade::Vibrante, 5)] {
            let lista = particulas(&[], &celulas, 100.0, intensidade);
            assert_eq!(lista.len(), quantas, "{intensidade:?}");
            assert!(lista.iter().all(|particula| particula.forma == Forma::Faisca));
            // Todas saem do meio da pedra do meio (a casa 12: linha 1, coluna 4) e são douradas.
            let (cx, cy) = centro(12);
            let perto_do_centro = lista.iter().all(|p| (p.x - cx).hypot(p.y - cy) <= 1.25 + 1e-4);
            assert!(perto_do_centro, "{intensidade:?}");
            assert!(lista.iter().all(|particula| particula.cor == DOURADO));
        }
        // Sem pedras e sem especiais, não há nada para mostrar.
        assert!(particulas(&[], &[], 100.0, PADRAO).is_empty());
    }

    #[test]
    fn pulso_e_raio_8_faiscas_e_espectro_12_no_padrao() {
        for (tipo, quantas) in [(TipoVisual::Pulso, 8), (TipoVisual::Raio, 8), (TipoVisual::Espectro, 12)] {
            for intensidade in [PADRAO, Intensidade::Vibrante] {
                let lista = particulas(&[visual(27, tipo)], &[27], 100.0, intensidade);
                assert_eq!(so(&lista, Forma::Faisca).len(), quantas, "{tipo:?} {intensidade:?}");
            }
        }
        // A Supernova também solta 8.
        let lista = particulas(&[visual(27, TipoVisual::Supernova)], &[27], 100.0, PADRAO);
        assert_eq!(so(&lista, Forma::Faisca).len(), 8);
    }

    #[test]
    fn suave_usa_5_faiscas_nas_especiais() {
        for tipo in [TipoVisual::Pulso, TipoVisual::Raio, TipoVisual::Espectro, TipoVisual::Supernova] {
            let lista = particulas(&[visual(27, tipo)], &[27], 100.0, Intensidade::Suave);
            assert_eq!(so(&lista, Forma::Faisca).len(), 5, "{tipo:?}");
        }
    }

    #[test]
    fn raio_tem_uma_onda_e_dois_feixes() {
        let lista = particulas(&[visual(27, TipoVisual::Raio)], &[27], 100.0, PADRAO);
        assert_eq!(so(&lista, Forma::Anel).len(), 1, "a onda");
        assert_eq!(so(&lista, Forma::Veu).len(), 0);
        let feixes = so(&lista, Forma::Linha);
        assert_eq!(feixes.len(), 2);
        let (cx, cy) = centro(27);
        // Um na horizontal (passa pela linha da pedra) e outro na vertical (pela coluna dela).
        let horizontal = feixes.iter().find(|f| f.angulo == 0.0).expect("um feixe horizontal");
        let vertical = feixes.iter().find(|f| (f.angulo - std::f32::consts::FRAC_PI_2).abs() < 1e-6).expect("um vertical");
        assert!((horizontal.y - cy).abs() < 1e-4 && (vertical.x - cx).abs() < 1e-4);
        // Aos 100 ms os feixes já cobrem o tabuleiro todo (de 0 a 8).
        for feixe in [horizontal, vertical] {
            assert!((feixe.tamanho - 8.0).abs() < 1e-3, "{feixe:?}");
            assert!((feixe.espessura - 0.2).abs() < 1e-4);
            assert!(feixe.alfa > 0.5);
        }
        assert!(horizontal.x.abs() < 1e-3 && vertical.y.abs() < 1e-3, "começam na borda do tabuleiro");
        // O Pulso só tem a onda (sem feixes).
        let pulso = particulas(&[visual(27, TipoVisual::Pulso)], &[27], 100.0, PADRAO);
        assert_eq!((so(&pulso, Forma::Anel).len(), so(&pulso, Forma::Linha).len()), (1, 0));
    }

    #[test]
    fn espectro_tem_veu_e_faiscas_de_cores_diferentes() {
        let lista = particulas(&[visual(27, TipoVisual::Espectro)], &[27], 100.0, PADRAO);
        let veu = so(&lista, Forma::Veu);
        assert_eq!(veu.len(), 1);
        assert!((veu[0].tamanho - 8.0).abs() < 1e-6 && veu[0].alfa > 0.0 && veu[0].alfa <= 0.9 * 0.4 + 1e-6);
        let faiscas = so(&lista, Forma::Faisca);
        assert_eq!(faiscas.len(), 12);
        for (i, faisca) in faiscas.iter().enumerate() {
            for outra in &faiscas[i + 1..] {
                assert_ne!(faisca.cor, outra.cor, "cada faísca do arco-íris tem uma cor");
            }
        }
        // O Raio tem faíscas azul-gelo; o Pulso, douradas.
        let raio = particulas(&[visual(27, TipoVisual::Raio)], &[27], 100.0, PADRAO);
        assert!(so(&raio, Forma::Faisca).iter().all(|f| f.cor == AZUL_GELO));
        let pulso = particulas(&[visual(27, TipoVisual::Pulso)], &[27], 100.0, PADRAO);
        assert!(so(&pulso, Forma::Faisca).iter().all(|f| f.cor == DOURADO));
    }

    #[test]
    fn supernova_tem_anel_de_5_casas_e_linhas_de_fusao_so_na_primeira_fase() {
        let nova = supernova(27); // origens 26, 27, 28
        // Primeira fase (240 ms): só as linhas de fusão, das duas pedras que não são o centro.
        let fusao = particulas(std::slice::from_ref(&nova), &[26, 27, 28], 100.0, PADRAO);
        assert_eq!(fusao.len(), 2);
        assert!(fusao.iter().all(|p| p.forma == Forma::Linha && p.cor == cor_da_nova(1) && (p.espessura - 0.08).abs() < 1e-6));
        let (cx, cy) = centro(27);
        for linha in &fusao {
            // A linha sai da pedra de origem e aponta para o centro.
            let (ox, oy) = (linha.x, linha.y);
            assert!(((cx - ox).atan2(cy - oy) - (linha.angulo.cos()).atan2(linha.angulo.sin())).abs() < 1e-4);
            assert!(linha.tamanho > 0.0 && linha.tamanho <= 1.0 + 1e-4, "cada origem está a 1 casa do centro");
        }
        // No fim da fusão as linhas chegaram ao centro.
        let quase_no_fim = particulas(std::slice::from_ref(&nova), &[26, 27, 28], 239.0, PADRAO);
        assert!(quase_no_fim.iter().all(|p| p.forma == Forma::Linha && (p.tamanho - 1.0).abs() < 0.05));

        // Segunda fase: o anel de 5 casas e nenhuma linha. Em 65% (eased) o anel está inteiro.
        let u_no_pico = 480.0 * (1.0 - 0.35_f32.powf(1.0 / 3.0));
        let explosao = particulas(&[nova], &[26, 27, 28], 240.0 + u_no_pico, PADRAO);
        assert!(so(&explosao, Forma::Linha).is_empty());
        let aneis = so(&explosao, Forma::Anel);
        assert_eq!(aneis.len(), 1);
        assert!((aneis[0].tamanho - 5.0).abs() < 1e-3 && aneis[0].cor == cor_da_nova(1));
        assert!((aneis[0].alfa - 0.7).abs() < 1e-3);
        assert!((aneis[0].x - cx).abs() < 1e-6 && (aneis[0].y - cy).abs() < 1e-6);
        assert_eq!(so(&explosao, Forma::Faisca).len(), 8);
    }

    #[test]
    fn no_maximo_3_especiais_alem_das_supernovas() {
        let cinco_pulsos: Vec<Visual> = (0..5).map(|i| visual(9 + 2 * i, TipoVisual::Pulso)).collect();
        let lista = particulas(&cinco_pulsos, &[], 100.0, PADRAO);
        assert_eq!(so(&lista, Forma::Anel).len(), 3, "uma onda por especial, só as 3 primeiras");
        assert_eq!(so(&lista, Forma::Faisca).len(), 24);
        // As três primeiras, na ordem do motor.
        let (primeiro_x, _) = centro(9);
        assert!(so(&lista, Forma::Anel).iter().any(|anel| (anel.x - primeiro_x).abs() < 1e-6));
        let (quarto_x, quarto_y) = centro(15);
        assert!(!so(&lista, Forma::Anel).iter().any(|anel| (anel.x - quarto_x).abs() < 1e-6 && (anel.y - quarto_y).abs() < 1e-6));
        // As Supernovas não entram na conta dos 3: uma Supernova e cinco pulsos dão o anel dela e três ondas.
        let mut com_nova = vec![visual(40, TipoVisual::Supernova)];
        com_nova.extend(cinco_pulsos);
        let lista = particulas(&com_nova, &[], 100.0, PADRAO);
        assert_eq!(so(&lista, Forma::Anel).len(), 4);
        assert_eq!(so(&lista, Forma::Faisca).len(), 32);
    }

    #[test]
    fn alfa_zero_no_comeco_e_particulas_somem_no_fim() {
        let conjuntos: Vec<(Vec<Visual>, f32)> = vec![
            (vec![], 380.0),
            (vec![visual(27, TipoVisual::Pulso)], 380.0),
            (vec![visual(27, TipoVisual::Raio), visual(28, TipoVisual::Espectro)], 380.0),
            (vec![supernova(27)], 240.0 + 480.0),
        ];
        for intensidade in Intensidade::TODAS {
            let escala = match intensidade {
                Intensidade::Suave => 0.82,
                Intensidade::Padrao => 1.0,
                Intensidade::Vibrante => 1.1,
            };
            for (visuais, total) in &conjuntos {
                let comeco = particulas(visuais, &[27], 0.0, intensidade);
                assert!(comeco.iter().all(|p| p.alfa == 0.0), "{intensidade:?}: tudo invisível em t = 0: {comeco:?}");
                let fim = particulas(visuais, &[27], total * escala + 1.0, intensidade);
                assert!(fim.is_empty(), "{intensidade:?}: nada depois do fim: {fim:?}");
                let no_meio = particulas(visuais, &[27], 0.4 * total * escala, intensidade);
                assert!(!no_meio.is_empty() && no_meio.iter().any(|p| p.alfa > 0.0), "{intensidade:?}: algo no meio");
            }
        }
    }

    #[test]
    fn tempo_estranho_nao_gera_nan_nem_passa_do_teto() {
        let visuais = vec![supernova(27), visual(9, TipoVisual::Raio), visual(10, TipoVisual::Espectro), visual(11, TipoVisual::Pulso)];
        for intensidade in Intensidade::TODAS {
            for t in [f32::NAN, -5.0, 1e30, f32::INFINITY, f32::NEG_INFINITY, -1e30] {
                let lista = particulas(&visuais, &[27, 28], t, intensidade);
                assert!(lista.len() <= MAXIMO_DE_PARTICULAS);
                for particula in &lista {
                    assert!(numeros(particula).iter().all(|n| n.is_finite()), "{intensidade:?} t = {t}: {particula:?}");
                }
                // Tempo enorme ou infinito: nada fica na tela; inválido ou negativo: começo (invisível).
                if t > 1000.0 {
                    assert!(lista.is_empty(), "t = {t}");
                } else {
                    assert!(lista.iter().all(|p| p.alfa == 0.0), "t = {t}");
                }
            }
            // NaN e tempo negativo valem o começo (0), e não "nada": mesmas partículas que em t = 0.
            let comeco = particulas(&visuais, &[27, 28], 0.0, intensidade);
            assert!(!comeco.is_empty());
            assert_eq!(particulas(&visuais, &[27, 28], f32::NAN, intensidade), comeco);
            assert_eq!(particulas(&visuais, &[27, 28], -5.0, intensidade), comeco);
        }
    }

    #[test]
    fn pior_caso_nao_passa_do_teto() {
        let oito_origens = |casa: usize, cor: u8| Visual {
            casa,
            tipo: TipoVisual::Supernova,
            cor,
            origens: vec![casa - 4, casa - 3, casa - 2, casa - 1, casa, casa + 1, casa + 2, casa + 3],
        };
        let visuais = vec![
            oito_origens(20, 0),
            oito_origens(44, 3),
            visual(9, TipoVisual::Espectro),
            visual(10, TipoVisual::Espectro),
            visual(11, TipoVisual::Espectro),
        ];
        let mut maior = 0;
        for intensidade in Intensidade::TODAS {
            for passo in 0..=160 {
                let t = passo as f32 * 6.0;
                let lista = particulas(&visuais, &[20, 44], t, intensidade);
                maior = maior.max(lista.len());
                assert!(lista.len() <= MAXIMO_DE_PARTICULAS, "{intensidade:?} t = {t}: {}", lista.len());
            }
        }
        assert!(maior >= 30, "o pior caso precisa ter muita partícula de verdade ({maior})");
        // Muitas ativações (mais do que cabe): o teto segura e as faíscas são as que caem fora.
        let muitas: Vec<Visual> = (0..40).map(|i| visual(10 + i, TipoVisual::Supernova)).collect();
        let lista = particulas(&muitas, &[], 100.0, PADRAO);
        assert_eq!(lista.len(), MAXIMO_DE_PARTICULAS);
        assert_eq!(so(&lista, Forma::Anel).len(), 40, "os anéis ficam; as faíscas é que são cortadas");
    }

    #[test]
    fn ajustar_suave_apaga_e_vibrante_realca() {
        let cor = [1.0, 0.5, 0.0];
        assert_eq!(ajustar(cor, 0.8, PADRAO), (cor, 0.8), "o Padrão não muda nada");
        // Suave: 48% do alfa e cor menos viva (as cores chegam mais perto uma da outra).
        let (suave, alfa) = ajustar(cor, 0.8, Intensidade::Suave);
        assert!((alfa - 0.8 * 0.48).abs() < 1e-6);
        let abertura = |c: [f32; 3]| c.iter().copied().fold(f32::MIN, f32::max) - c.iter().copied().fold(f32::MAX, f32::min);
        assert!(abertura(suave) < abertura(cor));
        // Vibrante: o alfa fica, e a cor fica mais clara (na média) e mais viva (o laranja vai para o extremo).
        let (vibrante, alfa) = ajustar(cor, 0.8, Intensidade::Vibrante);
        assert!((alfa - 0.8).abs() < 1e-6);
        assert!(vibrante[1] > cor[1], "o canal do meio sobe com o brilho: {vibrante:?}");
        assert!(vibrante.iter().all(|c| (0.0..=1.0).contains(c)), "sempre de 0 a 1");
        // Cores já claras ou escuras continuam de 0 a 1.
        for entrada in [[0.0, 0.0, 0.0], [1.0, 1.0, 1.0], [0.2, 0.9, 0.4]] {
            for intensidade in Intensidade::TODAS {
                let (saida, a) = ajustar(entrada, 1.0, intensidade);
                assert!(saida.iter().all(|c| (0.0..=1.0).contains(c)) && (0.0..=1.0).contains(&a));
            }
        }
    }

    #[test]
    fn faiscas_saem_em_angulos_de_2pi_i_sobre_n_mais_0_18() {
        for (tipo, n) in [(TipoVisual::Pulso, 8), (TipoVisual::Espectro, 12)] {
            let lista = particulas(&[visual(27, tipo)], &[27], 150.0, PADRAO);
            let faiscas = so(&lista, Forma::Faisca);
            assert_eq!(faiscas.len(), n);
            let (cx, cy) = centro(27);
            for (i, faisca) in faiscas.iter().enumerate() {
                let esperado = std::f32::consts::TAU * i as f32 / n as f32 + 0.18;
                assert!((faisca.angulo - esperado).abs() < 1e-5, "{tipo:?} faísca {i}");
                // E a posição realmente está nessa direção, a partir do centro da pedra.
                let direcao = (faisca.y - cy).atan2(faisca.x - cx);
                let diferenca = (direcao - esperado).rem_euclid(std::f32::consts::TAU);
                assert!(!(1e-3..=std::f32::consts::TAU - 1e-3).contains(&diferenca), "{tipo:?} faísca {i}: {direcao} vs {esperado}");
            }
        }
        // No Suave são 5 faíscas, e os ângulos acompanham o 5.
        let lista = particulas(&[visual(27, TipoVisual::Pulso)], &[27], 150.0, Intensidade::Suave);
        let faiscas = so(&lista, Forma::Faisca);
        assert!((faiscas[1].angulo - (std::f32::consts::TAU / 5.0 + 0.18)).abs() < 1e-5);
    }
}
