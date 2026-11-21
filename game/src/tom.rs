//! Gerador de tom (um "bipe") em formato WAV, feito na memória.
//!
//! No M0b serve para provar que o celular toca som. Mais adiante (M3) os efeitos do jogo
//! seguem o mesmo caminho: gerar as amostras em código, montar os bytes de um WAV e entregar
//! ao Macroquad, sem precisar de arquivo de som.

/// Amostras por segundo (a qualidade de um CD).
pub const TAXA: u32 = 44_100;

/// Duração máxima de um tom, para um valor errado não gerar um arquivo enorme.
const DURACAO_MAXIMA_MS: u32 = 5000;

/// Tempo de subida no começo e de descida no fim do tom. Sem isso o som começa e termina
/// "de repente" e dá um estalo nos alto-falantes.
const SUAVIZACAO_MS: u32 = 5;

/// Gera um tom puro (onda senoidal) e devolve os bytes de um arquivo WAV: 16 bits, um canal,
/// 44 100 amostras por segundo, com o cabeçalho padrão de 44 bytes.
///
/// Valores fora do razoável são corrigidos em vez de dar erro:
/// - `volume` fica entre 0 e 1 (e NaN vira 0, silêncio);
/// - `frequencia_hz` fica entre 20 e 20 000 (e NaN vira 440);
/// - `duracao_ms` fica entre 0 e 5000.
pub fn tom_wav(frequencia_hz: f32, duracao_ms: u32, volume: f32) -> Vec<u8> {
    let volume = if volume.is_nan() { 0.0 } else { volume.clamp(0.0, 1.0) };
    let frequencia = if frequencia_hz.is_nan() { 440.0 } else { frequencia_hz.clamp(20.0, 20_000.0) };
    let duracao_ms = duracao_ms.min(DURACAO_MAXIMA_MS);

    let total = (u64::from(duracao_ms) * u64::from(TAXA) / 1000) as usize;
    let suavizacao = (u64::from(SUAVIZACAO_MS) * u64::from(TAXA) / 1000) as usize;
    let bytes_de_dados = (total * 2) as u32;

    let mut wav = Vec::with_capacity(44 + total * 2);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + bytes_de_dados).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // tamanho deste bloco
    wav.extend_from_slice(&1u16.to_le_bytes()); // formato 1 = PCM
    wav.extend_from_slice(&1u16.to_le_bytes()); // um canal
    wav.extend_from_slice(&TAXA.to_le_bytes());
    wav.extend_from_slice(&(TAXA * 2).to_le_bytes()); // bytes por segundo
    wav.extend_from_slice(&2u16.to_le_bytes()); // bytes por amostra
    wav.extend_from_slice(&16u16.to_le_bytes()); // bits por amostra
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&bytes_de_dados.to_le_bytes());

    for i in 0..total {
        // `envelope` vai de 0 a 1 na subida, fica em 1 no meio e volta a 0 no fim.
        let subida = i as f32 / suavizacao as f32;
        let descida = (total - 1 - i) as f32 / suavizacao as f32;
        let envelope = subida.min(descida).min(1.0);
        // A fase é calculada em `f64`: em `f32`, tons agudos e longos perderiam precisão.
        let fase = 2.0 * std::f64::consts::PI * f64::from(frequencia) * i as f64 / f64::from(TAXA);
        let amostra = (fase.sin() as f32 * envelope * volume * 32767.0).round() as i16;
        wav.extend_from_slice(&amostra.to_le_bytes());
    }
    wav
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u16_em(wav: &[u8], posicao: usize) -> u16 {
        u16::from_le_bytes([wav[posicao], wav[posicao + 1]])
    }

    fn u32_em(wav: &[u8], posicao: usize) -> u32 {
        u32::from_le_bytes([wav[posicao], wav[posicao + 1], wav[posicao + 2], wav[posicao + 3]])
    }

    /// As amostras de som (os números de 16 bits depois do cabeçalho de 44 bytes).
    fn amostras(wav: &[u8]) -> Vec<i16> {
        wav[44..].chunks(2).map(|par| i16::from_le_bytes([par[0], par[1]])).collect()
    }

    fn pico(wav: &[u8]) -> i32 {
        amostras(wav).iter().map(|&amostra| i32::from(amostra).abs()).max().unwrap_or(0)
    }

    #[test]
    fn cabecalho_wav_esta_correto() {
        let wav = tom_wav(440.0, 300, 0.5);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        assert_eq!(u16_em(&wav, 20), 1, "formato 1 = PCM");
        assert_eq!(u16_em(&wav, 22), 1, "um canal");
        assert_eq!(u32_em(&wav, 24), 44_100, "taxa de amostragem");
        assert_eq!(u16_em(&wav, 34), 16, "bits por amostra");
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(u32_em(&wav, 4) as usize, wav.len() - 8, "tamanho do RIFF");
        assert_eq!(u32_em(&wav, 40) as usize, wav.len() - 44, "tamanho dos dados");
    }

    #[test]
    fn duracao_define_o_numero_de_amostras() {
        // 300 ms a 44 100 amostras por segundo = 13 230 amostras de 2 bytes cada.
        assert_eq!(tom_wav(440.0, 300, 0.5).len(), 44 + 2 * 13_230);
    }

    #[test]
    fn frequencia_certa_pelo_numero_de_cruzamentos_por_zero() {
        // Uma onda de 440 Hz cruza o zero 880 vezes por segundo.
        let mut cruzamentos = 0;
        let mut sinal_anterior = 0i32;
        for amostra in amostras(&tom_wav(440.0, 1000, 0.8)) {
            let sinal = i32::from(amostra).signum();
            if sinal != 0 {
                if sinal_anterior != 0 && sinal != sinal_anterior {
                    cruzamentos += 1;
                }
                sinal_anterior = sinal;
            }
        }
        assert!((876..=884).contains(&cruzamentos), "cruzamentos: {cruzamentos}");
    }

    #[test]
    fn volume_define_o_pico() {
        let metade = pico(&tom_wav(440.0, 300, 0.5)) as f32 / 32767.0;
        assert!((0.48..=0.52).contains(&metade), "pico com volume 0.5: {metade}");
        let cheio = pico(&tom_wav(440.0, 300, 1.0)) as f32 / 32767.0;
        assert!((0.95..=1.0).contains(&cheio), "pico com volume 1.0: {cheio}");
    }

    #[test]
    fn volume_fora_da_faixa_e_limitado() {
        assert_eq!(pico(&tom_wav(440.0, 300, 5.0)), pico(&tom_wav(440.0, 300, 1.0)));
        assert_eq!(pico(&tom_wav(440.0, 300, -1.0)), 0);
        assert_eq!(pico(&tom_wav(440.0, 300, f32::NAN)), 0);
    }

    #[test]
    fn duracao_zero_gera_wav_valido_sem_amostras() {
        let wav = tom_wav(440.0, 0, 0.5);
        assert_eq!(wav.len(), 44);
        assert_eq!(u32_em(&wav, 40), 0);
        assert_eq!(u32_em(&wav, 4), 36);
    }

    #[test]
    fn duracao_gigante_e_limitada() {
        assert_eq!(tom_wav(440.0, u32::MAX, 0.5).len(), 44 + 2 * 5 * 44_100);
    }

    #[test]
    fn comeca_e_termina_em_silencio() {
        let som = amostras(&tom_wav(440.0, 300, 0.9));
        assert_eq!(som.first(), Some(&0));
        assert_eq!(som.last(), Some(&0));
    }

    #[test]
    fn frequencia_invalida_nao_da_panico() {
        for frequencia in [0.0, -5.0, f32::NAN, f32::INFINITY] {
            let wav = tom_wav(frequencia, 100, 0.5);
            assert_eq!(wav.len(), 44 + 2 * 4_410, "frequência {frequencia}");
        }
    }
}
