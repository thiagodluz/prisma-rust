//! Efeitos sonoros do jogo, sintetizados em código: porte do `sound.js` do Prisma em JS.
//!
//! Cada efeito é uma lista de notas (frequência, quando começa, quanto dura, volume e forma da
//! onda). `renderizar` transforma a lista em amostras de som e `wav` entrega os bytes de um
//! arquivo WAV pronto para o Macroquad tocar. Tudo aqui é "puro": só números entram e saem, sem
//! tocar nada, então dá para testar sem placa de som.
//!
//! TEMPORÁRIO: o `allow` abaixo sai na Task 6, quando a camada de áudio passa a usar o módulo.
#![allow(dead_code)]

use crate::tom::{wav_de_amostras, TAXA};
use prisma_engine::TileKind;

/// Ganho com que os efeitos são tocados: `volume dos efeitos (0..1) × GANHO_MESTRE`. É o mesmo
/// 1,25 do JS, que deixa os efeitos um pouco acima da música no volume padrão.
pub const GANHO_MESTRE: f32 = 1.25;

/// Maior elevação de tom (em semitons) de uma cascata ou combinação.
const ELEVACAO_MAXIMA: u8 = 5;

/// Um som curto fica no máximo este tempo parado antes de começar e este tempo tocando. Só
/// serve de proteção contra valor errado; nenhum efeito de verdade chega perto.
const INICIO_MAXIMO_S: f32 = 2.0;
const DURACAO_MAXIMA_S: f32 = 3.0;

/// Sobra depois do fim da nota, igual à do JS (`stop(t + dur + 0.01)`).
const SOBRA_S: f64 = 0.010;

/// Volume "quase zero" do envelope: a rampa exponencial não chega em 0, só perto dele.
const SILENCIO: f64 = 0.0001;

/// Tempo máximo de subida do volume no começo da nota.
const SUBIDA_S: f64 = 0.018;

/// Os dez avisos sonoros do jogo.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sinal {
    /// Jogada que não vale: duas pedras trocaram e nada se formou.
    Invalida,
    /// Uma combinação simples.
    Combinacao,
    /// Combinação em cadeia (a 2ª, 3ª... seguida da mesma jogada).
    Cascata,
    /// Supernova: apagou o tabuleiro inteiro.
    Supernova,
    /// Pulso: explosão das 8 casas ao redor.
    Pulso,
    /// Raio: linha e coluna inteiras.
    Raio,
    /// Espectro: todas as pedras de uma cor.
    Espectro,
    /// Nasceu uma pedra especial.
    Criacao,
    /// Subiu de nível.
    Nivel,
    /// Resgate no modo Clássico.
    Resgate,
}

impl Sinal {
    /// O nome que aparece no registro (`PRISMA_M3 efeito sinal=<nome>`).
    pub fn nome(&self) -> &'static str {
        match self {
            Sinal::Invalida => "invalida",
            Sinal::Combinacao => "combinacao",
            Sinal::Cascata => "cascata",
            Sinal::Supernova => "supernova",
            Sinal::Pulso => "pulso",
            Sinal::Raio => "raio",
            Sinal::Espectro => "espectro",
            Sinal::Criacao => "criacao",
            Sinal::Nivel => "nivel",
            Sinal::Resgate => "resgate",
        }
    }
}

/// Um efeito a tocar: o sinal e quantos semitons o tom sobe (só combinação e cascata usam).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Efeito {
    pub sinal: Sinal,
    pub elevacao: u8,
}

/// Forma da onda de uma nota.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Onda {
    /// Som liso e redondo.
    Seno,
    /// Som um pouco mais "brilhante".
    Triangular,
}

/// Uma nota de um efeito. A frequência vai de `freq` a `freq_final` em rampa exponencial (se
/// forem iguais, a nota é fixa).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Nota {
    pub freq: f32,
    pub freq_final: f32,
    pub inicio_s: f32,
    pub duracao_s: f32,
    pub volume: f32,
    pub onda: Onda,
}

/// Nota fixa (a frequência não muda).
fn fixa(freq: f32, inicio_s: f32, duracao_s: f32, volume: f32, onda: Onda) -> Nota {
    Nota { freq, freq_final: freq, inicio_s, duracao_s, volume, onda }
}

/// Nota que desliza de `de` até `para`.
fn desliza(de: f32, para: f32, inicio_s: f32, duracao_s: f32, volume: f32, onda: Onda) -> Nota {
    Nota { freq: de, freq_final: para, inicio_s, duracao_s, volume, onda }
}

/// Uma sequência de notas fixas, a `passo_s` de distância uma da outra, começando em `comeco_s`.
fn arpejo(freqs: &[f32], comeco_s: f32, passo_s: f32, duracao_s: f32, volume: f32, onda: Onda) -> Vec<Nota> {
    freqs
        .iter()
        .enumerate()
        .map(|(i, &freq)| fixa(freq, comeco_s + i as f32 * passo_s, duracao_s, volume, onda))
        .collect()
}

/// Quantos semitons o tom sobe numa cadeia de `cadeia` combinações seguidas: a 1ª não sobe, cada
/// seguinte sobe um semitom, até no máximo 5.
pub fn elevacao_da_cadeia(cadeia: u32) -> u8 {
    cadeia.saturating_sub(1).min(u32::from(ELEVACAO_MAXIMA)) as u8
}

/// Qual sinal tocar numa limpeza do tabuleiro, na mesma prioridade do JS: supernova, depois
/// espectro, raio e pulso; sem nenhuma pedra especial, cascata se a cadeia passa de 1 e
/// combinação nos outros casos.
pub fn sinal_da_limpeza(supernova: bool, ativadas: &[TileKind], cadeia: u32) -> Sinal {
    if supernova {
        Sinal::Supernova
    } else if ativadas.contains(&TileKind::Spectrum) {
        Sinal::Espectro
    } else if ativadas.contains(&TileKind::Cross) {
        Sinal::Raio
    } else if ativadas.contains(&TileKind::Burst) {
        Sinal::Pulso
    } else if cadeia > 1 {
        Sinal::Cascata
    } else {
        Sinal::Combinacao
    }
}

/// A tabela de notas de um efeito, copiada do `sound.js`.
pub fn notas(efeito: Efeito) -> Vec<Nota> {
    use Onda::{Seno, Triangular};
    match efeito.sinal {
        Sinal::Invalida => vec![desliza(235.0, 205.0, 0.0, 0.09, 0.028, Triangular)],
        Sinal::Combinacao | Sinal::Cascata => {
            // Cada semitom multiplica a frequência por 2^(1/12).
            let fator = 2.0f32.powf(f32::from(efeito.elevacao.min(ELEVACAO_MAXIMA)) / 12.0);
            let volume = if efeito.sinal == Sinal::Cascata { 0.055 } else { 0.047 };
            let freqs = [523.25 * fator, 659.25 * fator, 783.99 * fator];
            arpejo(&freqs, 0.0, 0.057, 0.21, volume, Triangular)
        }
        Sinal::Supernova => {
            let mut notas = vec![
                desliza(130.81, 523.25, 0.0, 0.25, 0.06, Seno),
                desliza(220.0, 55.0, 0.24, 0.44, 0.09, Seno),
            ];
            notas.extend(arpejo(&[523.25, 783.99, 1046.5, 1567.98], 0.24, 0.045, 0.32, 0.045, Triangular));
            notas
        }
        Sinal::Pulso => {
            let mut notas = vec![desliza(190.0, 63.0, 0.0, 0.31, 0.085, Seno)];
            notas.extend(arpejo(&[740.0, 988.0, 1318.0], 0.045, 0.035, 0.17, 0.042, Triangular));
            notas
        }
        Sinal::Raio => arpejo(&[392.0, 587.33, 783.99, 1174.66], 0.0, 0.052, 0.32, 0.056, Seno),
        Sinal::Espectro => {
            let mut notas = arpejo(&[392.0, 440.0, 523.25, 587.33, 659.25, 783.99, 1046.5], 0.0, 0.046, 0.3, 0.042, Seno);
            notas.push(desliza(130.81, 261.63, 0.0, 0.53, 0.07, Triangular));
            notas
        }
        Sinal::Criacao => arpejo(&[659.25, 987.77], 0.0, 0.065, 0.29, 0.04, Seno),
        Sinal::Nivel => arpejo(&[392.0, 523.25, 659.25, 1046.5], 0.0, 0.11, 0.39, 0.055, Triangular),
        Sinal::Resgate => arpejo(&[587.33, 783.99, 1174.66], 0.0, 0.08, 0.35, 0.038, Seno),
    }
}

/// Os 20 efeitos que o jogo pode tocar: os 8 sinais fixos e, para combinação e cascata, as 6
/// elevações (0 a 5). A camada de áudio gera todos na hora de abrir o app.
pub fn todos() -> Vec<Efeito> {
    let mut lista = Vec::new();
    for sinal in [Sinal::Invalida, Sinal::Supernova, Sinal::Pulso, Sinal::Raio, Sinal::Espectro, Sinal::Criacao, Sinal::Nivel, Sinal::Resgate] {
        lista.push(Efeito { sinal, elevacao: 0 });
    }
    for sinal in [Sinal::Combinacao, Sinal::Cascata] {
        for elevacao in 0..=ELEVACAO_MAXIMA {
            lista.push(Efeito { sinal, elevacao });
        }
    }
    lista
}

/// Uma nota depois da limpeza de valores errados: tudo em `f64`, dentro dos limites.
struct NotaLimpa {
    freq: f64,
    razao: f64,
    inicio: f64,
    duracao: f64,
    volume: f64,
    onda: Onda,
}

/// Valores errados (NaN, infinito, duração zero, frequência ou volume que não passam de zero)
/// descartam a nota; início e duração grandes demais são limitados.
fn limpar(nota: &Nota) -> Option<NotaLimpa> {
    let finitos = [nota.freq, nota.freq_final, nota.inicio_s, nota.duracao_s, nota.volume];
    if finitos.iter().any(|v| !v.is_finite()) || nota.freq <= 0.0 || nota.freq_final <= 0.0 || nota.duracao_s <= 0.0 || nota.volume <= 0.0 {
        return None;
    }
    Some(NotaLimpa {
        freq: f64::from(nota.freq),
        razao: f64::from(nota.freq_final) / f64::from(nota.freq),
        inicio: f64::from(nota.inicio_s.clamp(0.0, INICIO_MAXIMO_S)),
        duracao: f64::from(nota.duracao_s.min(DURACAO_MAXIMA_S)),
        volume: f64::from(nota.volume).max(SILENCIO),
        onda: nota.onda,
    })
}

impl NotaLimpa {
    /// Fase da onda (em voltas) `u` segundos depois do começo da nota. A frequência desliza em
    /// rampa exponencial até o fim da nota e depois fica parada; a conta é a integral exata,
    /// então a fase não "escorrega" com o tempo.
    fn voltas(&self, u: f64) -> f64 {
        let ate = u.min(self.duracao);
        let voltas = if (self.razao - 1.0).abs() < 1e-9 {
            self.freq * ate
        } else {
            self.freq * self.duracao * (self.razao.powf(ate / self.duracao) - 1.0) / self.razao.ln()
        };
        voltas + self.freq * self.razao * (u - ate)
    }

    /// Volume `u` segundos depois do começo: sobe de "quase zero" até `volume` e depois desce
    /// de volta, as duas rampas exponenciais, e depois do fim da nota fica em "quase zero".
    fn ganho(&self, u: f64) -> f64 {
        let subida = SUBIDA_S.min(self.duracao / 4.0);
        if u < subida {
            SILENCIO * (self.volume / SILENCIO).powf(u / subida)
        } else if u < self.duracao {
            self.volume * (SILENCIO / self.volume).powf((u - subida) / (self.duracao - subida))
        } else {
            SILENCIO
        }
    }

    fn amostra(&self, u: f64) -> f64 {
        let fase = 2.0 * std::f64::consts::PI * self.voltas(u);
        let onda = match self.onda {
            Onda::Seno => fase.sin(),
            // Triangular: 2/π · asin(sin) sobe e desce em linha reta entre -1 e 1.
            Onda::Triangular => fase.sin().asin() * 2.0 / std::f64::consts::PI,
        };
        onda * self.ganho(u)
    }
}

/// Transforma as notas em amostras de som (um canal, 44 100 por segundo, valores entre -1 e 1).
/// O som dura até o fim da última nota mais 10 ms; sem nenhuma nota válida, não há som.
pub fn renderizar(notas: &[Nota]) -> Vec<f32> {
    let limpas: Vec<NotaLimpa> = notas.iter().filter_map(limpar).collect();
    let fim = limpas.iter().map(|n| n.inicio + n.duracao).fold(0.0, f64::max) + SOBRA_S;
    if limpas.is_empty() {
        return Vec::new();
    }
    let taxa = f64::from(TAXA);
    let total = (fim * taxa).round() as usize;
    let mut som = vec![0.0f32; total];
    for nota in &limpas {
        let primeira = (nota.inicio * taxa).ceil() as usize;
        let ultima = (((nota.inicio + nota.duracao + SOBRA_S) * taxa).ceil() as usize).min(total);
        for (j, amostra) in som.iter_mut().enumerate().take(ultima).skip(primeira) {
            *amostra += nota.amostra(j as f64 / taxa - nota.inicio) as f32;
        }
    }
    som
}

/// Os bytes de um arquivo WAV com o efeito pronto.
pub fn wav(efeito: Efeito) -> Vec<u8> {
    wav_de_amostras(&renderizar(&notas(efeito)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Os números do JS são de ponto flutuante: compara com uma folguinha.
    fn quase(a: f32, b: f32) -> bool {
        (a - b).abs() <= 1e-4 * b.abs().max(1.0)
    }

    fn nota(freq: f32, freq_final: f32, inicio_s: f32, duracao_s: f32, volume: f32, onda: Onda) -> Nota {
        Nota { freq, freq_final, inicio_s, duracao_s, volume, onda }
    }

    fn confere(real: &Nota, esperada: &Nota, onde: &str) {
        assert!(
            quase(real.freq, esperada.freq)
                && quase(real.freq_final, esperada.freq_final)
                && quase(real.inicio_s, esperada.inicio_s)
                && quase(real.duracao_s, esperada.duracao_s)
                && quase(real.volume, esperada.volume)
                && real.onda == esperada.onda,
            "{onde}: veio {real:?}, o JS tem {esperada:?}"
        );
    }

    fn sem_elevacao(sinal: Sinal) -> Efeito {
        Efeito { sinal, elevacao: 0 }
    }

    /// Quantas vezes a onda troca de sinal (de positivo para negativo ou o contrário).
    fn cruzamentos(amostras: &[f32]) -> usize {
        let mut anterior = 0.0f32;
        let mut conta = 0;
        for &amostra in amostras {
            if amostra != 0.0 {
                if anterior != 0.0 && (amostra > 0.0) != (anterior > 0.0) {
                    conta += 1;
                }
                anterior = amostra;
            }
        }
        conta
    }

    #[test]
    fn tabela_de_notas_bate_com_a_do_js() {
        use Onda::{Seno as S, Triangular as T};
        // (sinal, quantas notas, primeira, última), tirados do `sound.js` com elevação 0.
        let tabela = [
            (Sinal::Invalida, 1, nota(235.0, 205.0, 0.0, 0.09, 0.028, T), nota(235.0, 205.0, 0.0, 0.09, 0.028, T)),
            (Sinal::Combinacao, 3, nota(523.25, 523.25, 0.0, 0.21, 0.047, T), nota(783.99, 783.99, 0.114, 0.21, 0.047, T)),
            (Sinal::Cascata, 3, nota(523.25, 523.25, 0.0, 0.21, 0.055, T), nota(783.99, 783.99, 0.114, 0.21, 0.055, T)),
            (Sinal::Supernova, 6, nota(130.81, 523.25, 0.0, 0.25, 0.06, S), nota(1567.98, 1567.98, 0.375, 0.32, 0.045, T)),
            (Sinal::Pulso, 4, nota(190.0, 63.0, 0.0, 0.31, 0.085, S), nota(1318.0, 1318.0, 0.115, 0.17, 0.042, T)),
            (Sinal::Raio, 4, nota(392.0, 392.0, 0.0, 0.32, 0.056, S), nota(1174.66, 1174.66, 0.156, 0.32, 0.056, S)),
            (Sinal::Espectro, 8, nota(392.0, 392.0, 0.0, 0.3, 0.042, S), nota(130.81, 261.63, 0.0, 0.53, 0.07, T)),
            (Sinal::Criacao, 2, nota(659.25, 659.25, 0.0, 0.29, 0.04, S), nota(987.77, 987.77, 0.065, 0.29, 0.04, S)),
            (Sinal::Nivel, 4, nota(392.0, 392.0, 0.0, 0.39, 0.055, T), nota(1046.5, 1046.5, 0.33, 0.39, 0.055, T)),
            (Sinal::Resgate, 3, nota(587.33, 587.33, 0.0, 0.35, 0.038, S), nota(1174.66, 1174.66, 0.16, 0.35, 0.038, S)),
        ];
        for (sinal, quantas, primeira, ultima) in tabela {
            let notas = notas(sem_elevacao(sinal));
            assert_eq!(notas.len(), quantas, "{sinal:?}: quantidade de notas");
            confere(&notas[0], &primeira, &format!("{sinal:?} primeira"));
            confere(notas.last().unwrap(), &ultima, &format!("{sinal:?} última"));
        }
        // Os dois casos de duas partes, conferidos nota por nota.
        let supernova = notas(sem_elevacao(Sinal::Supernova));
        confere(&supernova[1], &nota(220.0, 55.0, 0.24, 0.44, 0.09, S), "Supernova segunda");
        confere(&supernova[2], &nota(523.25, 523.25, 0.24, 0.32, 0.045, T), "Supernova terceira");
        let pulso = notas(sem_elevacao(Sinal::Pulso));
        confere(&pulso[1], &nota(740.0, 740.0, 0.045, 0.17, 0.042, T), "Pulso segunda");
    }

    #[test]
    fn elevacao_5_multiplica_a_frequencia_por_2_elevado_a_5_sobre_12() {
        let notas = notas(Efeito { sinal: Sinal::Combinacao, elevacao: 5 });
        // 2^(5/12) = 1,33484
        assert!((notas[0].freq - 523.25 * 1.334_84).abs() < 0.01, "{}", notas[0].freq);
        assert!((notas[2].freq - 783.99 * 1.334_84).abs() < 0.01, "{}", notas[2].freq);
        // A elevação não mexe no tempo, no volume nem na forma.
        assert!(quase(notas[2].inicio_s, 0.114) && quase(notas[2].volume, 0.047) && notas[2].onda == Onda::Triangular);
    }

    #[test]
    fn elevacao_acima_de_5_vale_5_e_so_combinacao_e_cascata_usam_elevacao() {
        assert_eq!(
            notas(Efeito { sinal: Sinal::Cascata, elevacao: 9 }),
            notas(Efeito { sinal: Sinal::Cascata, elevacao: 5 })
        );
        for sinal in [Sinal::Invalida, Sinal::Supernova, Sinal::Pulso, Sinal::Raio, Sinal::Espectro, Sinal::Criacao, Sinal::Nivel, Sinal::Resgate] {
            assert_eq!(notas(Efeito { sinal, elevacao: 3 }), notas(sem_elevacao(sinal)), "{sinal:?}");
        }
    }

    #[test]
    fn elevacao_da_cadeia_limita_em_5() {
        for (cadeia, esperada) in [(0, 0), (1, 0), (2, 1), (3, 2), (6, 5), (7, 5), (1000, 5), (u32::MAX, 5)] {
            assert_eq!(elevacao_da_cadeia(cadeia), esperada, "cadeia {cadeia}");
        }
    }

    #[test]
    fn sinal_da_limpeza_segue_a_prioridade_do_js() {
        use prisma_engine::TileKind::{Burst, Cross, Normal, Spectrum};
        // Supernova ganha de tudo.
        assert_eq!(sinal_da_limpeza(true, &[Spectrum, Cross, Burst], 3), Sinal::Supernova);
        assert_eq!(sinal_da_limpeza(true, &[], 1), Sinal::Supernova);
        // Depois Espectro, Raio, Pulso (em qualquer ordem da lista).
        assert_eq!(sinal_da_limpeza(false, &[Burst, Cross, Spectrum], 3), Sinal::Espectro);
        assert_eq!(sinal_da_limpeza(false, &[Burst, Cross], 3), Sinal::Raio);
        assert_eq!(sinal_da_limpeza(false, &[Burst], 3), Sinal::Pulso);
        // Sem especial: cascata se a cadeia passa de 1, combinação nos outros casos.
        assert_eq!(sinal_da_limpeza(false, &[], 2), Sinal::Cascata);
        assert_eq!(sinal_da_limpeza(false, &[Normal], 5), Sinal::Cascata);
        assert_eq!(sinal_da_limpeza(false, &[], 1), Sinal::Combinacao);
        assert_eq!(sinal_da_limpeza(false, &[], 0), Sinal::Combinacao);
    }

    #[test]
    fn todos_sao_20_e_sem_repeticao() {
        let todos = todos();
        assert_eq!(todos.len(), 20);
        let mut unicos = todos.clone();
        unicos.sort_by_key(|e| (e.sinal.nome(), e.elevacao));
        unicos.dedup();
        assert_eq!(unicos.len(), 20);
        // Combinação e cascata têm as 6 elevações; os outros 8 só a 0.
        for sinal in [Sinal::Combinacao, Sinal::Cascata] {
            assert_eq!(todos.iter().filter(|e| e.sinal == sinal).count(), 6, "{sinal:?}");
        }
        assert_eq!(todos.iter().filter(|e| e.elevacao == 0).count(), 10);
    }

    #[test]
    fn os_nomes_dos_sinais_sao_os_do_registro() {
        let nomes: Vec<&str> = [
            Sinal::Invalida, Sinal::Combinacao, Sinal::Cascata, Sinal::Supernova, Sinal::Pulso,
            Sinal::Raio, Sinal::Espectro, Sinal::Criacao, Sinal::Nivel, Sinal::Resgate,
        ]
        .iter()
        .map(Sinal::nome)
        .collect();
        assert_eq!(
            nomes,
            ["invalida", "combinacao", "cascata", "supernova", "pulso", "raio", "espectro", "criacao", "nivel", "resgate"]
        );
    }

    #[test]
    fn duracao_do_som_e_o_fim_da_ultima_nota_mais_10_ms() {
        for efeito in todos() {
            let notas = notas(efeito);
            let fim = notas.iter().map(|n| f64::from(n.inicio_s) + f64::from(n.duracao_s)).fold(0.0, f64::max) + 0.010;
            let esperado = (fim * f64::from(crate::tom::TAXA)).round() as i64;
            let veio = renderizar(&notas).len() as i64;
            assert!((veio - esperado).abs() <= 1, "{efeito:?}: {veio} amostras, esperava {esperado}");
        }
    }

    #[test]
    fn pico_nao_passa_da_soma_dos_volumes_e_nao_e_mudo() {
        for efeito in todos() {
            let notas = notas(efeito);
            let soma: f32 = notas.iter().map(|n| n.volume).sum();
            let maior = notas.iter().map(|n| n.volume).fold(0.0, f32::max);
            let pico = renderizar(&notas).iter().fold(0.0f32, |m, a| m.max(a.abs()));
            assert!(pico <= soma + 0.001, "{efeito:?}: pico {pico} passa da soma {soma}");
            assert!(pico >= 0.3 * maior, "{efeito:?}: pico {pico} baixo demais (maior volume {maior})");
        }
    }

    #[test]
    fn o_som_comeca_e_termina_quase_em_silencio() {
        for efeito in todos() {
            let som = renderizar(&notas(efeito));
            assert!(som.first().unwrap().abs() <= 0.001, "{efeito:?}: começa com estalo");
            assert!(som.last().unwrap().abs() <= 0.001, "{efeito:?}: termina com estalo");
        }
    }

    #[test]
    fn nota_unica_de_440hz_tem_o_numero_certo_de_cruzamentos_por_zero() {
        let som = renderizar(&[nota(440.0, 440.0, 0.0, 0.5, 0.05, Onda::Seno)]);
        // 0,51 s de uma onda de 440 Hz: uns 449 cruzamentos por zero.
        let conta = cruzamentos(&som);
        assert!((445..=453).contains(&conta), "cruzamentos: {conta}");
    }

    #[test]
    fn varredura_de_190_para_63_desce() {
        let som = renderizar(&[nota(190.0, 63.0, 0.0, 0.31, 0.085, Onda::Seno)]);
        let por_50ms = (0.05 * f64::from(crate::tom::TAXA)) as usize;
        let fim_da_nota = (0.31 * f64::from(crate::tom::TAXA)) as usize;
        let no_comeco = cruzamentos(&som[..por_50ms]);
        let no_fim = cruzamentos(&som[fim_da_nota - por_50ms..fim_da_nota]);
        assert!(no_comeco >= 2 * no_fim, "começo {no_comeco} cruzamentos, fim {no_fim}");
        assert!(no_fim >= 3, "o fim ainda precisa ter som: {no_fim}");
    }

    #[test]
    fn onda_triangular_e_senoidal_diferem() {
        let seno = renderizar(&[nota(300.0, 300.0, 0.0, 0.5, 0.1, Onda::Seno)]);
        let triangular = renderizar(&[nota(300.0, 300.0, 0.0, 0.5, 0.1, Onda::Triangular)]);
        let rms = |som: &[f32]| (som.iter().map(|a| a * a).sum::<f32>() / som.len() as f32).sqrt();
        // Senoide: pico / raiz de 2. Triangular: pico / raiz de 3. A razão é 1,2247.
        let razao = rms(&seno) / rms(&triangular);
        assert!((1.19..=1.26).contains(&razao), "razão {razao}");
        let pico = |som: &[f32]| som.iter().fold(0.0f32, |m, a| m.max(a.abs()));
        assert!((pico(&seno) - pico(&triangular)).abs() < 0.005, "os dois chegam ao mesmo pico");
    }

    #[test]
    fn wav_dos_efeitos_e_valido_e_do_tamanho_do_som() {
        for efeito in todos() {
            let bytes = wav(efeito);
            let amostras = renderizar(&notas(efeito)).len();
            assert_eq!(&bytes[0..4], b"RIFF", "{efeito:?}");
            assert_eq!(&bytes[8..12], b"WAVE", "{efeito:?}");
            assert_eq!(bytes.len(), 44 + 2 * amostras, "{efeito:?}");
        }
    }

    #[test]
    fn nada_de_nota_nao_da_som() {
        assert!(renderizar(&[]).is_empty());
    }

    #[test]
    fn nota_com_valores_ruins_nao_da_panico() {
        // Duração zero ou negativa, frequência zero ou NaN, início negativo: sem pânico.
        for ruim in [
            nota(440.0, 440.0, 0.0, 0.0, 0.05, Onda::Seno),
            nota(440.0, 440.0, 0.0, -1.0, 0.05, Onda::Seno),
            nota(0.0, 0.0, 0.0, 0.1, 0.05, Onda::Seno),
            nota(f32::NAN, 440.0, 0.0, 0.1, 0.05, Onda::Triangular),
            nota(440.0, 440.0, -0.5, 0.1, 0.05, Onda::Seno),
            nota(440.0, 440.0, f32::NAN, 0.1, 0.05, Onda::Seno),
            nota(440.0, 440.0, 0.0, f32::INFINITY, 0.05, Onda::Seno),
        ] {
            let som = renderizar(&[ruim]);
            assert!(som.iter().all(|a| a.is_finite()), "{ruim:?}");
            assert!(som.len() <= 6 * crate::tom::TAXA as usize, "{ruim:?}: som grande demais");
        }
    }
}
