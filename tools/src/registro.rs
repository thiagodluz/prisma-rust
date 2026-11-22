//! Leitura do registro (o "logcat") do celular virtual.
//!
//! O jogo escreve no registro do Android linhas como `PRISMA_M1 jogada valida=true pontos=60 nivel=1`
//! (o que o jogador vê e faz) e, desde o M2, linhas `PRISMA_M2 ...` (o que o app guardou e leu do
//! armazenamento: `aberto`, `gravado`, `erro_ao_gravar`).
//! Este módulo transforma cada uma dessas linhas num valor de Rust (`Linha`), sem depender de
//! `grep` nem de `sed`. É código puro (só texto entra e sai), por isso é fácil de testar.
//!
//! Uma linha que não dá para ler (falta um campo, o número é inválido, o tipo não existe) é
//! simplesmente ignorada: quem espera por ela vai falhar com uma mensagem clara por não achá-la.

use std::str::FromStr;

/// De onde o app tirou a partida de um modo ao abrir (o `zen=<origem>:<pontos>` da linha `aberto`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origem {
    /// Não havia nada gravado: partida nova.
    Novo,
    /// Leu o arquivo gravado.
    Arquivo,
    /// O arquivo estava estragado: partida nova (e o arquivo foi guardado como `.corrupt`).
    Estragado,
}

impl Origem {
    /// Como a origem aparece no registro (`novo`, `arquivo` ou `estragado`).
    pub fn nome(self) -> &'static str {
        match self {
            Origem::Novo => "novo",
            Origem::Arquivo => "arquivo",
            Origem::Estragado => "estragado",
        }
    }
}

/// `PRISMA_M2 aberto modo=zen zen=arquivo:375 classico=novo:0 melhor_zen=375 melhor_classico=0`:
/// com o que o app abriu. Cada modo vem com a origem da partida e os pontos dela.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Aberto {
    pub modo: String,
    pub zen: (Origem, u64),
    pub classico: (Origem, u64),
    pub melhor_zen: u64,
    pub melhor_classico: u64,
}

/// Escreve a linha como o app a escreve: assim as mensagens de erro do teste mostram o que o app disse.
impl std::fmt::Display for Aberto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PRISMA_M2 aberto modo={} zen={}:{} classico={}:{} melhor_zen={} melhor_classico={}",
            self.modo,
            self.zen.0.nome(),
            self.zen.1,
            self.classico.0.nome(),
            self.classico.1,
            self.melhor_zen,
            self.melhor_classico
        )
    }
}

/// Uma linha `PRISMA_M1 ...` ou `PRISMA_M2 ...` do registro, já lida.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Linha {
    /// `PRISMA_M1 pronto modo=zen`: o jogo abriu.
    Pronto { modo: String },
    /// `PRISMA_M1 tabuleiro x=32 y=612 lado=1016`: onde o tabuleiro está (pixels da janela do app).
    Tabuleiro { x: i32, y: i32, lado: i32 },
    /// `PRISMA_M1 botao=dica x=540 y=1830`: o centro de um botão.
    Botao { nome: String, x: i32, y: i32 },
    /// `PRISMA_M1 toque x=160 y=36`: onde o app sentiu um toque.
    Toque { x: i32, y: i32 },
    /// `PRISMA_M1 dica a=17 b=18`: as duas casas que a dica mostrou.
    Dica { a: usize, b: usize },
    /// `PRISMA_M1 jogada valida=true pontos=60 nivel=1`.
    Jogada { valida: bool, pontos: u64, nivel: u32 },
    /// `PRISMA_M1 animacao_terminou`.
    AnimacaoTerminou,
    /// `PRISMA_M1 modo=classico pontos=0`: o jogo mudou de modo.
    Modo { modo: String, pontos: u64 },
    /// `PRISMA_M1 novo_jogo modo=classico pontos=0`.
    NovoJogo { modo: String, pontos: u64 },
    /// `PRISMA_M1 fim pontos=4321 nivel=3`: a partida acabou.
    Fim { pontos: u64, nivel: u32 },
    /// `PRISMA_M2 aberto ...`: o que o app leu do armazenamento ao abrir.
    Aberto(Aberto),
    /// `PRISMA_M2 gravado modo=zen pontos=75`: a partida do modo foi gravada.
    Gravado { modo: String, pontos: u64 },
    /// `PRISMA_M2 erro_ao_gravar <texto>`: a gravação falhou (o jogo segue).
    ErroAoGravar { texto: String },
}

/// Os marcadores que o jogo põe no começo de cada linha dele. O logcat escreve hora, número do
/// processo etc. ANTES deles, então o marcador pode aparecer no meio da linha.
const MARCADOR_M1: &str = "PRISMA_M1 ";
const MARCADOR_M2: &str = "PRISMA_M2 ";

/// Qual marcador a linha tem (vale o que aparece primeiro) e o texto que vem depois dele.
fn depois_do_marcador(texto: &str) -> Option<(&'static str, &str)> {
    [MARCADOR_M1, MARCADOR_M2]
        .into_iter()
        .filter_map(|marcador| Some((texto.find(marcador)?, marcador)))
        .min_by_key(|(posicao, _)| *posicao)
        .map(|(posicao, marcador)| (marcador, &texto[posicao + marcador.len()..]))
}

/// `novo:0`, `arquivo:375` ou `estragado:0` (a origem e os pontos da partida de um modo).
fn origem_e_pontos(palavras: &[&str], chave: &str) -> Option<(Origem, u64)> {
    let (origem, pontos) = campo(palavras, chave)?.split_once(':')?;
    let origem = match origem {
        "novo" => Origem::Novo,
        "arquivo" => Origem::Arquivo,
        "estragado" => Origem::Estragado,
        _ => return None,
    };
    Some((origem, pontos.parse().ok()?))
}

/// Acha o valor de `chave=valor` entre as palavras da linha. Valor vazio conta como ausente.
fn campo<'a>(palavras: &[&'a str], chave: &str) -> Option<&'a str> {
    palavras
        .iter()
        .find_map(|p| p.strip_prefix(chave)?.strip_prefix('='))
        .filter(|valor| !valor.is_empty())
}

/// Acha `chave=valor` e converte o valor (`i32`, `u64`...). Número inválido, negativo onde só cabe
/// positivo ou grande demais viram `None`, nunca pânico.
fn numero<T: FromStr>(palavras: &[&str], chave: &str) -> Option<T> {
    campo(palavras, chave)?.parse().ok()
}

/// Lê uma linha do registro. `None` se não tem o marcador ou não dá para entender o resto.
pub fn ler_linha(texto: &str) -> Option<Linha> {
    let (marcador, depois) = depois_do_marcador(texto)?;
    if marcador == MARCADOR_M2 {
        return ler_linha_m2(depois);
    }
    let palavras: Vec<&str> = depois.split_whitespace().collect();
    let primeira = *palavras.first()?;

    // `botao=<nome>` e `modo=<modo>` trazem o valor colado na primeira palavra.
    if let Some(nome) = primeira.strip_prefix("botao=") {
        return Some(Linha::Botao {
            nome: Some(nome).filter(|n| !n.is_empty())?.to_string(),
            x: numero(&palavras, "x")?,
            y: numero(&palavras, "y")?,
        });
    }
    if let Some(modo) = primeira.strip_prefix("modo=") {
        return Some(Linha::Modo {
            modo: Some(modo).filter(|m| !m.is_empty())?.to_string(),
            pontos: numero(&palavras, "pontos")?,
        });
    }
    match primeira {
        "pronto" => Some(Linha::Pronto { modo: campo(&palavras, "modo")?.to_string() }),
        "tabuleiro" => Some(Linha::Tabuleiro {
            x: numero(&palavras, "x")?,
            y: numero(&palavras, "y")?,
            lado: numero(&palavras, "lado")?,
        }),
        "toque" => Some(Linha::Toque { x: numero(&palavras, "x")?, y: numero(&palavras, "y")? }),
        "dica" => Some(Linha::Dica { a: numero(&palavras, "a")?, b: numero(&palavras, "b")? }),
        "jogada" => Some(Linha::Jogada {
            valida: match campo(&palavras, "valida")? {
                "true" => true,
                "false" => false,
                _ => return None,
            },
            pontos: numero(&palavras, "pontos")?,
            nivel: numero(&palavras, "nivel")?,
        }),
        "animacao_terminou" => Some(Linha::AnimacaoTerminou),
        "novo_jogo" => Some(Linha::NovoJogo {
            modo: campo(&palavras, "modo")?.to_string(),
            pontos: numero(&palavras, "pontos")?,
        }),
        "fim" => Some(Linha::Fim { pontos: numero(&palavras, "pontos")?, nivel: numero(&palavras, "nivel")? }),
        _ => None,
    }
}

/// Lê o que vem depois de `PRISMA_M2 `. Só entende as linhas que o teste do celular usa; as outras
/// (como `sem_pasta`) ficam de fora.
fn ler_linha_m2(depois: &str) -> Option<Linha> {
    let palavras: Vec<&str> = depois.split_whitespace().collect();
    match *palavras.first()? {
        "aberto" => Some(Linha::Aberto(Aberto {
            modo: campo(&palavras, "modo")?.to_string(),
            zen: origem_e_pontos(&palavras, "zen")?,
            classico: origem_e_pontos(&palavras, "classico")?,
            melhor_zen: numero(&palavras, "melhor_zen")?,
            melhor_classico: numero(&palavras, "melhor_classico")?,
        })),
        "gravado" => Some(Linha::Gravado {
            modo: campo(&palavras, "modo")?.to_string(),
            pontos: numero(&palavras, "pontos")?,
        }),
        // O texto do erro é o resto da linha, com espaços e tudo.
        "erro_ao_gravar" => {
            let texto = depois.trim().strip_prefix("erro_ao_gravar")?.trim();
            Some(Linha::ErroAoGravar { texto: Some(texto).filter(|t| !t.is_empty())?.to_string() })
        }
        _ => None,
    }
}

/// Todas as linhas `PRISMA_M1` e `PRISMA_M2` de um despejo do logcat, na ordem em que apareceram.
#[derive(Debug)]
pub struct Registro {
    linhas: Vec<Linha>,
}

impl Registro {
    /// Lê um despejo inteiro do logcat. Linhas de outros programas e linhas ilegíveis são ignoradas.
    pub fn de(logcat: &str) -> Registro {
        Registro { linhas: logcat.lines().filter_map(ler_linha).collect() }
    }

    /// Onde o tabuleiro está: `(x, y, lado)`. Vale a última linha (o app escreve de novo a cada mudança).
    pub fn ultimo_tabuleiro(&self) -> Option<(i32, i32, i32)> {
        self.linhas.iter().rev().find_map(|l| match l {
            Linha::Tabuleiro { x, y, lado } => Some((*x, *y, *lado)),
            _ => None,
        })
    }

    /// O centro do botão `nome`, da última linha dele.
    pub fn ultimo_botao(&self, nome: &str) -> Option<(i32, i32)> {
        self.linhas.iter().rev().find_map(|l| match l {
            Linha::Botao { nome: n, x, y } if n == nome => Some((*x, *y)),
            _ => None,
        })
    }

    /// O último toque que o app sentiu: `(x, y)`.
    pub fn ultimo_toque(&self) -> Option<(i32, i32)> {
        self.linhas.iter().rev().find_map(|l| match l {
            Linha::Toque { x, y } => Some((*x, *y)),
            _ => None,
        })
    }

    /// As duas casas da última dica.
    pub fn ultima_dica(&self) -> Option<(usize, usize)> {
        self.linhas.iter().rev().find_map(|l| match l {
            Linha::Dica { a, b } => Some((*a, *b)),
            _ => None,
        })
    }

    /// A última jogada: `(valida, pontos, nivel)`.
    pub fn ultima_jogada(&self) -> Option<(bool, u64, u32)> {
        self.linhas.iter().rev().find_map(|l| match l {
            Linha::Jogada { valida, pontos, nivel } => Some((*valida, *pontos, *nivel)),
            _ => None,
        })
    }

    /// A última troca de modo: `(modo, pontos)`.
    pub fn ultimo_modo(&self) -> Option<(String, u64)> {
        self.linhas.iter().rev().find_map(|l| match l {
            Linha::Modo { modo, pontos } => Some((modo.clone(), *pontos)),
            _ => None,
        })
    }

    pub fn contar_animacoes(&self) -> usize {
        self.linhas.iter().filter(|l| matches!(l, Linha::AnimacaoTerminou)).count()
    }

    pub fn contar_dicas(&self) -> usize {
        self.linhas.iter().filter(|l| matches!(l, Linha::Dica { .. })).count()
    }

    pub fn contar_toques(&self) -> usize {
        self.linhas.iter().filter(|l| matches!(l, Linha::Toque { .. })).count()
    }

    /// O jogo avisou que abriu no modo `modo`?
    pub fn tem_pronto(&self, modo: &str) -> bool {
        self.linhas.iter().any(|l| matches!(l, Linha::Pronto { modo: m } if m == modo))
    }

    /// O último `aberto`: com o que o app abriu da última vez (depois de um `logcat -c`, só vale a
    /// abertura mais recente).
    pub fn ultimo_aberto(&self) -> Option<Aberto> {
        self.linhas.iter().rev().find_map(|l| match l {
            Linha::Aberto(aberto) => Some(aberto.clone()),
            _ => None,
        })
    }

    /// O app gravou a partida desse modo com exatamente esses pontos?
    pub fn tem_gravado(&self, modo: &str, pontos: u64) -> bool {
        self.linhas
            .iter()
            .any(|l| matches!(l, Linha::Gravado { modo: m, pontos: p } if m == modo && *p == pontos))
    }

    /// O texto do primeiro `erro_ao_gravar`, se o app falhou ao gravar.
    pub fn primeiro_erro_ao_gravar(&self) -> Option<String> {
        self.linhas.iter().find_map(|l| match l {
            Linha::ErroAoGravar { texto } => Some(texto.clone()),
            _ => None,
        })
    }

    /// Existe a linha `novo_jogo` com esse modo e esses pontos?
    pub fn tem_novo_jogo(&self, modo: &str, pontos: u64) -> bool {
        self.linhas
            .iter()
            .any(|l| matches!(l, Linha::NovoJogo { modo: m, pontos: p } if m == modo && *p == pontos))
    }
}

/// O centro da casa `casa` (0 a 63, linha por linha) em coordenadas da TELA, que é o que o
/// `adb input` usa. O app mede a partir do canto da própria janela; `deslocamento` é a diferença.
/// Mesma conta do script antigo: `x + dx + (2*coluna + 1) * lado / 16`, com divisão inteira
/// depois de multiplicar. Números enormes viram o limite do `i32` em vez de dar pânico.
pub fn centro_da_casa(tabuleiro: (i32, i32, i32), deslocamento: (i32, i32), casa: usize) -> (i32, i32) {
    let (x, y, lado) = tabuleiro;
    // `i128` cabe qualquer multiplicação aqui; só no fim volta para `i32`, limitando nas pontas.
    let ponto = |base: i32, desloc: i32, indice: usize| {
        let meio = (2 * indice as i128 + 1) * i128::from(lado) / 16;
        let total = i128::from(base) + i128::from(desloc) + meio;
        i32::try_from(total).unwrap_or(if total < 0 { i32::MIN } else { i32::MAX })
    };
    (ponto(x, deslocamento.0, casa % 8), ponto(y, deslocamento.1, casa / 8))
}

/// A diferença entre onde o toque foi dado na tela e onde o app o viu: `tela - visto`, em x e em y.
pub fn deslocamento(tela: (i32, i32), visto: (i32, i32)) -> (i32, i32) {
    (tela.0.saturating_sub(visto.0), tela.1.saturating_sub(visto.1))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// As linhas de logcat que o fluxo real do M1 imprimiu no GitHub (celular virtual, Android 11):
    /// 70 linhas do jogo (`PRISMA_M1 ...`) misturadas com 130 de outros programas. Para o M2 foram
    /// acrescentadas à mão 3 linhas `PRISMA_M2` (`aberto`, `gravado` e `erro_ao_gravar`), no formato
    /// que o app escreve, nos lugares onde ele as escreveria.
    const AMOSTRA: &str = include_str!("../tests/amostra-logcat.txt");

    fn tipo_de(linha: &Linha) -> &'static str {
        match linha {
            Linha::Pronto { .. } => "pronto",
            Linha::Tabuleiro { .. } => "tabuleiro",
            Linha::Botao { .. } => "botao",
            Linha::Toque { .. } => "toque",
            Linha::Dica { .. } => "dica",
            Linha::Jogada { .. } => "jogada",
            Linha::AnimacaoTerminou => "animacao",
            Linha::Modo { .. } => "modo",
            Linha::NovoJogo { .. } => "novo_jogo",
            Linha::Fim { .. } => "fim",
            Linha::Aberto(_) => "aberto",
            Linha::Gravado { .. } => "gravado",
            Linha::ErroAoGravar { .. } => "erro_ao_gravar",
        }
    }

    fn aberto(modo: &str, zen: (Origem, u64), classico: (Origem, u64), melhores: (u64, u64)) -> Aberto {
        Aberto { modo: modo.to_string(), zen, classico, melhor_zen: melhores.0, melhor_classico: melhores.1 }
    }

    fn quantas(registro: &Registro, tipo: &str) -> usize {
        registro.linhas.iter().filter(|l| tipo_de(l) == tipo).count()
    }

    #[test]
    fn le_todas_as_linhas_da_amostra_real() {
        let r = Registro::de(AMOSTRA);
        // Contagens conferidas à mão com `grep -c "PRISMA_M1 <tipo>"` no arquivo (70 do M1 + 3 do M2).
        assert_eq!(r.linhas.len(), 73);
        assert_eq!(quantas(&r, "pronto"), 1);
        assert_eq!(quantas(&r, "tabuleiro"), 2);
        assert_eq!(quantas(&r, "botao"), 15);
        assert_eq!(quantas(&r, "toque"), 24);
        assert_eq!(quantas(&r, "dica"), 9);
        assert_eq!(quantas(&r, "jogada"), 9);
        assert_eq!(quantas(&r, "animacao"), 9);
        assert_eq!(quantas(&r, "modo"), 1);
        assert_eq!(r.contar_toques(), 24);
        assert_eq!(r.contar_dicas(), 9);
        assert_eq!(r.contar_animacoes(), 9);
        // Valem as últimas linhas (a geometria é escrita de novo a cada mudança de modo).
        assert_eq!(r.ultimo_tabuleiro(), Some((10, 194, 301)));
        assert_eq!(r.ultimo_botao("dica"), Some((109, 517)));
        assert_eq!(r.ultimo_botao("embaralhar"), Some((58, 517)));
        assert_eq!(r.ultimo_botao("confirmar"), Some((221, 346)));
        assert_eq!(r.ultimo_botao("inexistente"), None);
        assert_eq!(r.ultimo_toque(), Some((216, 250)));
        assert_eq!(r.ultima_dica(), Some((12, 13)));
        assert_eq!(r.ultima_jogada(), Some((true, 405, 1)));
        assert_eq!(r.ultimo_modo(), Some(("classico".to_string(), 0)));
        assert!(r.tem_pronto("zen"));
        assert!(!r.tem_pronto("classico"));
        // O resumo do CI é cortado em 200 linhas: o `novo_jogo` do fim do fluxo não está na amostra.
        assert!(!r.tem_novo_jogo("classico", 0));
    }

    #[test]
    fn cada_tipo_de_linha_e_lido_com_e_sem_o_prefixo_do_logcat() {
        let prefixo = "11-21 20:36:19.088  3597  3633 I SAPP    : ";
        let casos: Vec<(&str, Linha)> = vec![
            ("PRISMA_M1 pronto modo=zen", Linha::Pronto { modo: "zen".into() }),
            ("PRISMA_M1 tabuleiro x=32 y=612 lado=1016", Linha::Tabuleiro { x: 32, y: 612, lado: 1016 }),
            ("PRISMA_M1 botao=dica x=540 y=1830", Linha::Botao { nome: "dica".into(), x: 540, y: 1830 }),
            ("PRISMA_M1 toque x=160 y=36", Linha::Toque { x: 160, y: 36 }),
            ("PRISMA_M1 dica a=17 b=18", Linha::Dica { a: 17, b: 18 }),
            ("PRISMA_M1 jogada valida=true pontos=60 nivel=1", Linha::Jogada { valida: true, pontos: 60, nivel: 1 }),
            ("PRISMA_M1 jogada valida=false pontos=0 nivel=1", Linha::Jogada { valida: false, pontos: 0, nivel: 1 }),
            ("PRISMA_M1 animacao_terminou", Linha::AnimacaoTerminou),
            ("PRISMA_M1 modo=classico pontos=0", Linha::Modo { modo: "classico".into(), pontos: 0 }),
            ("PRISMA_M1 novo_jogo modo=classico pontos=0", Linha::NovoJogo { modo: "classico".into(), pontos: 0 }),
            ("PRISMA_M1 fim pontos=4321 nivel=3", Linha::Fim { pontos: 4321, nivel: 3 }),
            (
                "PRISMA_M2 aberto modo=zen zen=novo:0 classico=novo:0 melhor_zen=0 melhor_classico=0",
                Linha::Aberto(aberto("zen", (Origem::Novo, 0), (Origem::Novo, 0), (0, 0))),
            ),
            (
                "PRISMA_M2 aberto modo=classico zen=arquivo:375 classico=estragado:0 melhor_zen=400 melhor_classico=120",
                Linha::Aberto(aberto("classico", (Origem::Arquivo, 375), (Origem::Estragado, 0), (400, 120))),
            ),
            ("PRISMA_M2 gravado modo=zen pontos=75", Linha::Gravado { modo: "zen".into(), pontos: 75 }),
            (
                "PRISMA_M2 erro_ao_gravar não consegui gravar zen.json: disco cheio",
                Linha::ErroAoGravar { texto: "não consegui gravar zen.json: disco cheio".into() },
            ),
        ];
        for (texto, esperada) in casos {
            assert_eq!(ler_linha(texto), Some(esperada.clone()), "sem prefixo: {texto}");
            let com_prefixo = format!("{prefixo}{texto}");
            assert_eq!(ler_linha(&com_prefixo), Some(esperada), "com prefixo: {com_prefixo}");
        }
    }

    #[test]
    fn vale_a_ultima_linha_de_geometria() {
        let r = Registro::de(
            "PRISMA_M1 tabuleiro x=10 y=20 lado=300\n\
             PRISMA_M1 botao=dica x=160 y=517\n\
             PRISMA_M1 tabuleiro x=11 y=21 lado=301\n\
             PRISMA_M1 botao=dica x=109 y=517\n\
             PRISMA_M1 botao=novo_jogo x=211 y=517\n",
        );
        assert_eq!(r.ultimo_tabuleiro(), Some((11, 21, 301)));
        assert_eq!(r.ultimo_botao("dica"), Some((109, 517)));
        assert_eq!(r.ultimo_botao("novo_jogo"), Some((211, 517)));
    }

    #[test]
    fn linhas_repetidas_contam_cada_uma_como_no_grep() {
        let r = Registro::de("PRISMA_M1 dica a=1 b=2\nPRISMA_M1 dica a=1 b=2\nPRISMA_M1 toque x=1 y=2\nPRISMA_M1 toque x=1 y=2\nPRISMA_M1 toque x=1 y=2\nPRISMA_M1 animacao_terminou\nPRISMA_M1 animacao_terminou\n");
        assert_eq!(r.contar_dicas(), 2);
        assert_eq!(r.contar_toques(), 3);
        assert_eq!(r.contar_animacoes(), 2);
    }

    #[test]
    fn linhas_estranhas_sao_ignoradas_sem_pane() {
        let estranhas = [
            "PRISMA_M1 tabuleiro x=1 y=2",                      // falta o lado
            "PRISMA_M1 toque x=1.5 y=2",                        // número com vírgula
            "PRISMA_M1 toque x=abc y=2",                        // não é número
            "PRISMA_M1 toque x=99999999999999999999 y=2",       // grande demais
            "PRISMA_M1 dica a=-1 b=2",                          // casa negativa
            "PRISMA_M1 jogada valida=talvez pontos=1 nivel=1",  // nem true nem false
            "PRISMA_M1 jogada valida=true pontos=-5 nivel=1",   // pontos negativos
            "PRISMA_M1 botao= x=1 y=2",                         // botão sem nome
            "PRISMA_M1 modo=classico pontos=",                  // valor vazio
            "PRISMA_M1 inventada x=1",                          // tipo que não existe
            "PRISMA_M1 pronto",                                 // falta o modo
            "PRISMA_M1 toque x=1 y",                            // cortada no meio
            "PRISMA_M1",                                        // só o marcador
            "PRISMA_M1 ",                                       // marcador e nada mais
            "PRISMAM1 toque x=1 y=2",                           // marcador errado
            "I AndroidRuntime: VM exiting with result code 0.", // outro programa
            "",
            "   ",
        ];
        for texto in estranhas {
            assert_eq!(ler_linha(texto), None, "devia ser ignorada: {texto:?}");
        }
        // Misturadas com linhas boas, só as boas entram, com \r\n e espaços a mais.
        let r = Registro::de("lixo\r\nPRISMA_M1 toque x=1.5 y=2\r\n  PRISMA_M1   toque   x=3   y=4  \r\nPRISMA_M1 toque x=5\r\n");
        assert_eq!(r.contar_toques(), 1);
        assert_eq!(r.ultimo_toque(), Some((3, 4)));
        // Texto vazio e bytes estranhos (já convertidos) também não dão pane.
        assert_eq!(Registro::de("").contar_toques(), 0);
        assert_eq!(Registro::de("\u{fffd}\u{fffd} PRISMA_M1 \u{fffd}").contar_toques(), 0);
    }

    #[test]
    fn le_as_linhas_prisma_m2_da_amostra() {
        let r = Registro::de(AMOSTRA);
        assert_eq!(quantas(&r, "aberto"), 1);
        assert_eq!(quantas(&r, "gravado"), 1);
        assert_eq!(quantas(&r, "erro_ao_gravar"), 1);
        assert_eq!(r.ultimo_aberto(), Some(aberto("zen", (Origem::Novo, 0), (Origem::Novo, 0), (0, 0))));
        assert!(r.tem_gravado("zen", 195));
        assert!(!r.tem_gravado("zen", 196), "os pontos têm de ser exatamente esses");
        assert!(!r.tem_gravado("classico", 195), "o modo também");
        assert_eq!(
            r.primeiro_erro_ao_gravar().as_deref(),
            Some("não consegui criar a pasta /data/data/x/files: Permission denied (os error 13)")
        );
        // As linhas do M2 não atrapalham as do M1.
        assert_eq!(r.ultima_jogada(), Some((true, 405, 1)));
        assert!(r.tem_pronto("zen"));
    }

    #[test]
    fn vale_o_ultimo_aberto_e_sem_linha_nao_ha_resposta() {
        let r = Registro::de(
            "PRISMA_M2 aberto modo=zen zen=novo:0 classico=novo:0 melhor_zen=0 melhor_classico=0\n\
             PRISMA_M2 aberto modo=classico zen=arquivo:375 classico=arquivo:75 melhor_zen=375 melhor_classico=300\n\
             PRISMA_M2 gravado modo=classico pontos=75\n\
             PRISMA_M2 gravado modo=zen pontos=375\n",
        );
        assert_eq!(
            r.ultimo_aberto(),
            Some(aberto("classico", (Origem::Arquivo, 375), (Origem::Arquivo, 75), (375, 300)))
        );
        assert!(r.tem_gravado("classico", 75));
        assert!(r.tem_gravado("zen", 375));
        assert!(!r.tem_gravado("zen", 75));
        assert_eq!(r.primeiro_erro_ao_gravar(), None);
        let vazio = Registro::de("PRISMA_M1 pronto modo=zen\n");
        assert_eq!(vazio.ultimo_aberto(), None);
        assert!(!vazio.tem_gravado("zen", 0));
        assert_eq!(vazio.primeiro_erro_ao_gravar(), None);
    }

    #[test]
    fn linha_m2_mal_formada_ou_com_origem_desconhecida_e_ignorada() {
        let estranhas = [
            "PRISMA_M2 aberto modo=zen zen=novo:0 classico=novo:0 melhor_zen=0", // falta o melhor_classico
            "PRISMA_M2 aberto modo=zen zen=inventado:0 classico=novo:0 melhor_zen=0 melhor_classico=0", // origem que não existe
            "PRISMA_M2 aberto modo=zen zen=novo classico=novo:0 melhor_zen=0 melhor_classico=0", // sem os pontos
            "PRISMA_M2 aberto modo=zen zen=novo:abc classico=novo:0 melhor_zen=0 melhor_classico=0", // pontos que não são número
            "PRISMA_M2 aberto modo=zen zen=novo:-1 classico=novo:0 melhor_zen=0 melhor_classico=0", // pontos negativos
            "PRISMA_M2 aberto modo=zen zen=novo:0:5 classico=novo:0 melhor_zen=0 melhor_classico=0", // dois-pontos demais
            "PRISMA_M2 aberto modo=zen zen=:0 classico=novo:0 melhor_zen=0 melhor_classico=0", // origem vazia
            "PRISMA_M2 aberto zen=novo:0 classico=novo:0 melhor_zen=0 melhor_classico=0", // falta o modo
            "PRISMA_M2 aberto modo=zen zen=novo:0 classico=novo:0 melhor_zen=x melhor_classico=0", // recorde que não é número
            "PRISMA_M2 gravado modo=zen",                                        // falta o pontos
            "PRISMA_M2 gravado modo=zen pontos=x",                               // pontos que não são número
            "PRISMA_M2 gravado pontos=5",                                        // falta o modo
            "PRISMA_M2 erro_ao_gravar",                                          // sem texto
            "PRISMA_M2 erro_ao_gravar    ",                                      // só espaços
            "PRISMA_M2 sem_pasta: não achei a pasta",                            // linha que o teste não usa
            "PRISMA_M2 inventada x=1",                                           // tipo que não existe
            "PRISMA_M2",                                                         // só o marcador
            "PRISMA_M2 ",                                                        // marcador e nada mais
            "PRISMA_M3 gravado modo=zen pontos=5",                               // marcador que não existe
            "PRISMAM2 gravado modo=zen pontos=5",                                // marcador errado
            "PRISMA_M1 gravado modo=zen pontos=5",                               // tipo do M2 com o marcador do M1
            "PRISMA_M2 toque x=1 y=2",                                           // tipo do M1 com o marcador do M2
        ];
        for texto in estranhas {
            assert_eq!(ler_linha(texto), None, "devia ser ignorada: {texto:?}");
        }
        // Misturadas com linhas boas, só as boas entram.
        let r = Registro::de(
            "PRISMA_M2 gravado modo=zen pontos=x\r\n  PRISMA_M2   gravado   modo=zen   pontos=75  \r\nPRISMA_M2 aberto modo=zen\r\n",
        );
        assert!(r.tem_gravado("zen", 75));
        assert_eq!(r.ultimo_aberto(), None);
    }

    #[test]
    fn o_aberto_se_escreve_do_mesmo_jeito_que_o_app_para_aparecer_nas_mensagens_de_erro() {
        let linhas = [
            "PRISMA_M2 aberto modo=classico zen=arquivo:375 classico=estragado:0 melhor_zen=400 melhor_classico=120",
            "PRISMA_M2 aberto modo=zen zen=novo:0 classico=novo:0 melhor_zen=0 melhor_classico=0",
        ];
        for linha in linhas {
            let Some(Linha::Aberto(lido)) = ler_linha(linha) else { panic!("devia ler: {linha}") };
            assert_eq!(lido.to_string(), linha);
        }
        assert_eq!(Origem::Arquivo.nome(), "arquivo");
        assert_eq!(Origem::Novo.nome(), "novo");
        assert_eq!(Origem::Estragado.nome(), "estragado");
    }

    #[test]
    fn centro_da_casa_bate_com_a_conta_do_shell() {
        // tabuleiro em (100, 300) com lado 800; a janela começa 28 px abaixo do topo da tela.
        let tabuleiro = (100, 300, 800);
        let deslocamento = (0, 28);
        assert_eq!(centro_da_casa(tabuleiro, deslocamento, 0), (150, 378));
        assert_eq!(centro_da_casa(tabuleiro, deslocamento, 9), (250, 478));
        assert_eq!(centro_da_casa(tabuleiro, deslocamento, 63), (850, 1078));
        // Divisão inteira DEPOIS de multiplicar, como no shell: (2*7+1)*815/16 = 764, e não 15*(815/16) = 750.
        assert_eq!(centro_da_casa((100, 300, 815), (0, 0), 7), (864, 350));
    }

    #[test]
    fn centro_da_casa_com_numeros_enormes_nao_da_pane() {
        let (x, y) = centro_da_casa((i32::MAX, i32::MAX, i32::MAX), (i32::MAX, i32::MAX), 63);
        assert_eq!((x, y), (i32::MAX, i32::MAX));
        let (x, y) = centro_da_casa((i32::MIN, i32::MIN, i32::MAX), (i32::MIN, i32::MIN), 0);
        assert_eq!((x, y), (i32::MIN, i32::MIN));
        let _ = centro_da_casa((1, 2, 3), (4, 5), usize::MAX);
    }

    #[test]
    fn deslocamento_e_a_diferenca_entre_a_tela_e_o_que_o_app_viu() {
        assert_eq!(deslocamento((160, 60), (160, 36)), (0, 24));
        assert_eq!(deslocamento((100, 60), (90, 20)), (10, 40));
        assert_eq!(deslocamento((i32::MIN, i32::MAX), (i32::MAX, i32::MIN)), (i32::MIN, i32::MAX));
    }
}
