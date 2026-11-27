//! A camada fina com o Macroquad: carrega e toca os sons. Aqui não se decide nada.
//!
//! - Os 20 efeitos são gerados na memória (`efeitos`), carregados uma vez na abertura e tocados
//!   por quem chama (`tocar`).
//! - A música e o ambiente do Zen são arquivos `.ogg`: o `fundo::planejar` diz o que fazer
//!   (carregar, parar, mudar o volume) e `aplicar` executa, uma faixa por canal.
//!
//! Regra de ouro: som nunca derruba o jogo. A biblioteca de áudio "dá pânico" (fecha o programa)
//! se o arquivo estiver cortado ou se não houver placa de som; por isso toda chamada a ela vai
//! dentro de um `catch_unwind` (que segura o pânico e o transforma em erro comum), como o
//! `tocar_som` do M0b. Um erro vira uma linha `PRISMA_M3 erro ...` no registro e um aviso na folha
//! de Som; o jogo continua.

use crate::app::App;
use crate::efeitos::{self, Efeito, GANHO_MESTRE};
use crate::fundo::{Atual, Comando, Faixa};
use crate::preferencias::{Canal, Volumes};
use macroquad::audio::{load_sound_from_bytes, play_sound, set_sound_volume, stop_sound, PlaySoundParams, Sound};
// Atenção: o `info!` do miniquad não aceita `{nome}` dentro do texto; os valores vão como argumentos.
use macroquad::logging::info;
use std::collections::HashMap;
use std::future::Future;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::pin::pin;
use std::task::{Context, Poll, Waker};
use std::time::Instant;

/// O volume de saída de um efeito: o volume dos efeitos (0 a 100) dividido por 100 e vezes o
/// "ganho mestre" do jogo em JS (1,25). Com o volume em 100 o efeito sai 25% mais alto que o
/// arquivo; com 80 (o padrão) sai com o volume do arquivo.
fn volume_dos_efeitos(volumes: &Volumes) -> f32 {
    (f32::from(volumes.efeitos) / 100.0 * GANHO_MESTRE).max(0.0)
}

/// O volume de uma faixa de fundo: sempre entre 0,0 e 1,0 (número inválido vira silêncio).
fn volume_de_fundo(volume: f32) -> f32 {
    if volume.is_nan() {
        0.0
    } else {
        volume.clamp(0.0, 1.0)
    }
}

/// Uma linha de registro é uma linha só: troca as quebras de linha do texto por espaço.
fn uma_linha(texto: &str) -> String {
    texto.replace(['\n', '\r'], " ").trim().to_string()
}

/// Roda um `async` que não tem nada para esperar (o carregamento do Macroquad, fora da web, termina
/// na primeira olhada) e devolve o resultado. Existe para poder pôr o carregamento dentro de um
/// `catch_unwind`, que não funciona com `.await`. `None` = o futuro ficou esperando (não deveria).
fn pronto<T>(futuro: impl Future<Output = T>) -> Option<T> {
    // O `Waker::noop` é um "acordador" que não faz nada: aqui ninguém precisa ser acordado.
    let mut contexto = Context::from_waker(Waker::noop());
    let mut futuro = pin!(futuro);
    match futuro.as_mut().poll(&mut contexto) {
        Poll::Ready(valor) => Some(valor),
        Poll::Pending => None,
    }
}

/// Decodifica os bytes de um áudio (WAV ou OGG). Arquivo vazio, cortado ou estragado, ou áudio
/// indisponível, viram `Err` com um texto; nunca um pânico.
fn carregar_bytes(bytes: &[u8]) -> Result<Sound, String> {
    match catch_unwind(AssertUnwindSafe(|| pronto(load_sound_from_bytes(bytes)))) {
        Ok(Some(Ok(som))) => Ok(som),
        Ok(Some(Err(erro))) => Err(format!("áudio ilegível: {erro:?}")),
        Ok(None) => Err("o áudio não terminou de carregar".to_string()),
        Err(_) => Err("arquivo de áudio estragado ou áudio indisponível".to_string()),
    }
}

/// A pasta dos sons no PC: a da variável `PRISMA_SONS` (se tiver valor) ou a pasta `sons` ao lado
/// do executável. Sem saber onde o executável está, a `sons` do diretório atual.
#[cfg(not(target_os = "android"))]
fn pasta_dos_sons_com(variavel: Option<std::ffi::OsString>, executavel: Option<std::path::PathBuf>) -> std::path::PathBuf {
    match variavel {
        Some(pasta) if !pasta.is_empty() => std::path::PathBuf::from(pasta),
        _ => {
            let ao_lado = executavel.as_deref().and_then(std::path::Path::parent);
            ao_lado.unwrap_or_else(|| std::path::Path::new(".")).join("sons")
        }
    }
}

/// Lê os bytes de uma faixa no PC: direto do disco, na pasta dos sons.
#[cfg(not(target_os = "android"))]
async fn ler_faixa(caminho: &str) -> Result<Vec<u8>, String> {
    let pasta = pasta_dos_sons_com(std::env::var_os("PRISMA_SONS"), std::env::current_exe().ok());
    let arquivo = pasta.join(caminho);
    std::fs::read(&arquivo).map_err(|erro| format!("não consegui ler {}: {erro}", arquivo.display()))
}

/// Lê os bytes de uma faixa no Android: dos "assets" do APK (a pasta de sons vai dentro dele).
#[cfg(target_os = "android")]
async fn ler_faixa(caminho: &str) -> Result<Vec<u8>, String> {
    macroquad::file::load_file(caminho).await.map_err(|erro| format!("não consegui ler {caminho}: {erro:?}"))
}

/// Lê e decodifica uma faixa de fundo (caminho relativo à pasta dos sons).
async fn carregar_faixa(caminho: &str) -> Result<Sound, String> {
    let bytes = ler_faixa(caminho).await?;
    if bytes.is_empty() {
        return Err(format!("o arquivo {caminho} está vazio"));
    }
    carregar_bytes(&bytes)
}

/// A faixa que está carregada num canal: qual foi, com que volume, e o som em si (`None` se o
/// carregamento falhou: a faixa fica anotada para não ser tentada de novo à toa).
struct Vaga {
    faixa: Faixa,
    volume: f32,
    som: Option<Sound>,
}

/// Todos os sons do jogo: os 20 efeitos e, no máximo, uma música e um ambiente.
pub struct Audio {
    efeitos: HashMap<Efeito, Sound>,
    musica: Option<Vaga>,
    ambiente: Option<Vaga>,
}

impl Audio {
    /// Gera e carrega os 20 efeitos (leva uma fração de segundo) e anota o tempo no registro.
    pub fn novo() -> Audio {
        let inicio = Instant::now();
        let mut efeitos = HashMap::new();
        for efeito in efeitos::todos() {
            match carregar_bytes(&efeitos::wav(efeito)) {
                Ok(som) => {
                    efeitos.insert(efeito, som);
                }
                Err(erro) => info!("PRISMA_M3 erro tipo=efeito nome={} texto={}", efeito.sinal.nome(), uma_linha(&erro)),
            }
        }
        info!("PRISMA_M3 pronto efeitos={} ms={}", efeitos.len(), inicio.elapsed().as_millis());
        Audio { efeitos, musica: None, ambiente: None }
    }

    /// Toca um efeito, uma vez, com o volume dos efeitos. Efeito desconhecido ou áudio com problema:
    /// não toca nada e o jogo segue.
    pub fn tocar(&self, efeito: Efeito, volumes: &Volumes) {
        let Some(som) = self.efeitos.get(&efeito) else { return };
        let volume = volume_dos_efeitos(volumes);
        let _ = catch_unwind(AssertUnwindSafe(|| play_sound(som, PlaySoundParams { looped: false, volume })));
    }

    /// O que está carregado agora (inclusive uma faixa que falhou), para o `fundo::planejar`.
    pub fn atual(&self) -> Atual {
        let musica = match &self.musica {
            Some(Vaga { faixa: Faixa::Musica(faixa), volume, .. }) => Some((*faixa, *volume)),
            _ => None,
        };
        let ambiente = match &self.ambiente {
            Some(Vaga { faixa: Faixa::Ambiente(faixa), volume, .. }) => Some((*faixa, *volume)),
            _ => None,
        };
        Atual { musica, ambiente }
    }

    /// Executa os comandos do planejador, na ordem. Falha ao carregar não para nada: vira
    /// `app.erro_de_audio` (o aviso da folha) e uma linha no registro.
    pub async fn aplicar(&mut self, comandos: Vec<Comando>, app: &mut App) {
        for comando in comandos {
            match comando {
                Comando::Parar(canal) => {
                    self.parar(canal);
                    app.erro_de_audio(canal, None);
                }
                Comando::Carregar(faixa, volume) => self.carregar(faixa, volume, app).await,
                Comando::Volume(canal, volume) => self.mudar_volume(canal, volume),
            }
        }
    }

    fn vaga(&mut self, canal: Canal) -> Option<&mut Option<Vaga>> {
        match canal {
            Canal::Musica => Some(&mut self.musica),
            Canal::Ambiente => Some(&mut self.ambiente),
            Canal::Efeitos => None,
        }
    }

    /// Para o canal e solta a faixa da memória.
    fn parar(&mut self, canal: Canal) {
        let Some(vaga) = self.vaga(canal).and_then(Option::take) else { return };
        if let Some(som) = vaga.som {
            // O `Sound` apaga a si mesmo da memória quando é solto (no fim deste bloco).
            let _ = catch_unwind(AssertUnwindSafe(move || stop_sound(&som)));
        }
    }

    fn mudar_volume(&mut self, canal: Canal, volume: f32) {
        let volume = volume_de_fundo(volume);
        let Some(Some(vaga)) = self.vaga(canal) else { return };
        vaga.volume = volume;
        if let Some(som) = &vaga.som {
            let _ = catch_unwind(AssertUnwindSafe(|| set_sound_volume(som, volume)));
        }
    }

    /// Carrega a faixa e toca em laço. A que estava no canal (se ainda estiver) é solta ANTES.
    async fn carregar(&mut self, faixa: Faixa, volume: f32, app: &mut App) {
        let canal = faixa.canal();
        self.parar(canal);
        let volume = volume_de_fundo(volume);
        let inicio = Instant::now();
        let resultado = match carregar_faixa(faixa.caminho()).await {
            Ok(som) => {
                let tocou = catch_unwind(AssertUnwindSafe(|| play_sound(&som, PlaySoundParams { looped: true, volume }))).is_ok();
                if tocou {
                    Ok(som)
                } else {
                    Err("o áudio não respondeu".to_string())
                }
            }
            Err(erro) => Err(erro),
        };
        let som = match resultado {
            Ok(som) => {
                info!("PRISMA_M3 carregado tipo={} nome={} ms={}", faixa.tipo(), faixa.nome(), inicio.elapsed().as_millis());
                app.erro_de_audio(canal, None);
                Some(som)
            }
            Err(erro) => {
                info!("PRISMA_M3 erro tipo={} nome={} texto={}", faixa.tipo(), faixa.nome(), uma_linha(&erro));
                app.erro_de_audio(canal, Some(erro));
                None
            }
        };
        if let Some(vaga) = self.vaga(canal) {
            *vaga = Some(Vaga { faixa, volume, som });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faixa_ausente_vira_erro_e_nao_panico() {
        // Um caminho que não existe na pasta dos sons: `Err` com texto (e o nome do arquivo), sem
        // pânico e sem precisar de placa de som nem de janela.
        let resultado = pronto(carregar_faixa("ambiente/nao-existe-de-jeito-nenhum.ogg")).expect("não espera nada");
        let erro = resultado.expect_err("o arquivo não existe");
        assert!(erro.contains("nao-existe-de-jeito-nenhum"), "o erro diz qual arquivo: {erro}");
        assert!(!erro.contains('\n'));
    }

    #[test]
    fn arquivo_vazio_ou_estragado_vira_erro_e_nao_panico() {
        // Sem a janela do Macroquad o pânico vem do contexto que falta, mas é o mesmo caminho que um
        // OGG cortado percorre: o pânico de dentro da biblioteca é segurado e vira `Err`.
        for bytes in [&b""[..], b"isto nao e um ogg", &[0u8; 64]] {
            let erro = carregar_bytes(bytes).expect_err("não é um áudio");
            assert!(!erro.is_empty());
        }
    }

    #[test]
    fn volume_dos_efeitos_usa_o_ganho_mestre() {
        let com = |efeitos| volume_dos_efeitos(&Volumes { efeitos, musica: 70, ambiente: 65 });
        assert_eq!(com(0), 0.0);
        assert!((com(80) - 1.0).abs() < 1e-6, "o padrão (80) toca no volume do arquivo");
        assert!((com(100) - 1.25).abs() < 1e-6, "100 = ganho mestre cheio");
        assert!((com(40) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn volume_de_fundo_fica_entre_0_e_1_e_nan_vira_silencio() {
        assert_eq!(volume_de_fundo(0.65), 0.65);
        assert_eq!(volume_de_fundo(-3.0), 0.0);
        assert_eq!(volume_de_fundo(7.0), 1.0);
        assert_eq!(volume_de_fundo(f32::NAN), 0.0);
        assert_eq!(volume_de_fundo(f32::INFINITY), 1.0);
    }

    #[test]
    fn texto_de_erro_vira_uma_linha_so() {
        assert_eq!(uma_linha("falhou\nde novo\r\n"), "falhou de novo");
        assert_eq!(uma_linha("tudo numa linha"), "tudo numa linha");
    }

    #[cfg(not(target_os = "android"))]
    #[test]
    fn a_pasta_dos_sons_no_pc_e_a_da_variavel_ou_a_sons_ao_lado_do_executavel() {
        use std::ffi::OsString;
        use std::path::PathBuf;
        let exe = || Some(PathBuf::from("/jogos/prisma/prisma_game"));
        let ao_lado = PathBuf::from("/jogos/prisma/sons");
        assert_eq!(pasta_dos_sons_com(None, exe()), ao_lado);
        assert_eq!(pasta_dos_sons_com(Some(OsString::new()), exe()), ao_lado, "variável vazia vale como ausente");
        assert_eq!(
            pasta_dos_sons_com(Some(OsString::from("/tmp/meus-sons")), exe()),
            PathBuf::from("/tmp/meus-sons"),
            "a variável vence a pasta ao lado do executável"
        );
    }

    #[cfg(not(target_os = "android"))]
    #[test]
    fn sem_saber_onde_esta_o_executavel_a_pasta_dos_sons_e_a_sons_do_diretorio_atual() {
        assert_eq!(pasta_dos_sons_com(None, None), std::path::PathBuf::from("./sons"));
    }
}
