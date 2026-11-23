//! Versão do Android da camada da plataforma.
//!
//! O Android é feito em Java; o nosso jogo é Rust. JNI é a "ponte" que deixa o Rust chamar
//! métodos Java. Aqui chamamos dois deles: `getFilesDir()` (a pasta privada do app) e o
//! `Vibrator` (o vibrador do aparelho: ver se existe e vibrar por um tempo ou com um padrão).
//! Usamos os ajudantes `ndk_utils` do miniquad, que escondem a parte mais chata da ponte.
//!
//! Este arquivo só compila para Android, então só o GitHub (que monta o APK) e o celular
//! provam que funciona; não dá para rodá-lo no PC.
//!
//! Dois cuidados valem para toda chamada:
//! - Se o Java "joga uma exceção" (um erro), ela fica pendente e derruba o app na próxima
//!   chamada. Por isso, depois de cada chamada, `limpar_excecao` confere e limpa.
//! - Cada objeto Java que recebemos ocupa uma "referência local". Fora de uma chamada que
//!   veio do Java, elas só somem quando a thread acaba; como a nossa não acaba, abrimos um
//!   "quadro" de referências (`com_quadro`) e o fechamos no fim, para não acumular.

use std::ffi::CString;
use std::path::PathBuf;

use macroquad::miniquad::native::android::ndk_utils::{
    call_bool_method, call_object_method, call_void_method, get_utf_str,
};
use macroquad::miniquad::native::android::{attach_jni_env, ndk_sys, ACTIVITY};

/// Se o Java deixou uma exceção pendente, mostra no registro, limpa e devolve `true`.
unsafe fn limpar_excecao(env: *mut ndk_sys::JNIEnv) -> bool {
    let pendente = (**env).ExceptionCheck.unwrap()(env) != 0;
    if pendente {
        (**env).ExceptionDescribe.unwrap()(env);
        (**env).ExceptionClear.unwrap()(env);
    }
    pendente
}

/// Roda `trabalho` dentro de um quadro de referências locais, que é fechado no fim
/// (qualquer que seja o resultado). O que `trabalho` devolve precisa ser dado do Rust
/// (texto, número), nunca uma referência Java, porque elas somem com o quadro.
unsafe fn com_quadro<T>(
    env: *mut ndk_sys::JNIEnv,
    trabalho: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    if (**env).PushLocalFrame.unwrap()(env, 16) != 0 {
        return Err("o Android não deu memória para a chamada".to_string());
    }
    let resultado = trabalho();
    (**env).PopLocalFrame.unwrap()(env, std::ptr::null_mut());
    resultado
}

/// Pasta privada do app (`Context.getFilesDir().getAbsolutePath()`).
pub fn pasta_de_dados() -> Result<PathBuf, String> {
    unsafe {
        let env = attach_jni_env();
        com_quadro(env, || {
            let pasta = call_object_method!(env, ACTIVITY, "getFilesDir", "()Ljava/io/File;");
            if limpar_excecao(env) || pasta.is_null() {
                return Err("o Android não deu a pasta de dados do app (getFilesDir)".to_string());
            }
            let caminho =
                call_object_method!(env, pasta, "getAbsolutePath", "()Ljava/lang/String;");
            if limpar_excecao(env) || caminho.is_null() {
                return Err("o Android não deu o caminho da pasta de dados".to_string());
            }
            Ok(PathBuf::from(get_utf_str!(env, caminho)))
        })
    }
}

/// O que o jogo diz quando o aparelho não tem vibrador.
const SEM_VIBRADOR: &str = "este aparelho não tem vibrador";

/// O objeto `Vibrator` do sistema (`Context.getSystemService("vibrator")`), se o aparelho tiver
/// vibrador de verdade (`hasVibrator`). Só vale dentro de `com_quadro`, que fecha a referência.
unsafe fn vibrador_do_aparelho(env: *mut ndk_sys::JNIEnv) -> Result<ndk_sys::jobject, String> {
    let nome = CString::new("vibrator").unwrap();
    let texto = (**env).NewStringUTF.unwrap()(env, nome.as_ptr());
    let vibrador = call_object_method!(
        env,
        ACTIVITY,
        "getSystemService",
        "(Ljava/lang/String;)Ljava/lang/Object;",
        texto
    );
    if limpar_excecao(env) || vibrador.is_null() {
        return Err(SEM_VIBRADOR.to_string());
    }
    let existe = call_bool_method!(env, vibrador, "hasVibrator", "()Z");
    if limpar_excecao(env) || existe == 0 {
        return Err(SEM_VIBRADOR.to_string());
    }
    Ok(vibrador)
}

/// O aparelho tem vibrador? Qualquer erro da ponte vale "não" (o interruptor some, o jogo segue).
pub fn tem_vibrador() -> bool {
    unsafe {
        let env = attach_jni_env();
        com_quadro(env, || vibrador_do_aparelho(env).map(|_| ())).is_ok()
    }
}

/// Faz o aparelho vibrar por `milissegundos` (`Context.getSystemService("vibrator")`).
///
/// Usa `Vibrator.vibrate(long)`, que o Android marca como antigo desde a versão 8, mas
/// continua funcionando em todas as versões e dispensa o código extra da versão nova.
pub fn vibrar(milissegundos: u32) -> Result<(), String> {
    unsafe {
        let env = attach_jni_env();
        com_quadro(env, || {
            let vibrador = vibrador_do_aparelho(env)?;
            call_void_method!(env, vibrador, "vibrate", "(J)V", ndk_sys::jlong::from(milissegundos));
            if limpar_excecao(env) {
                return Err("o Android recusou a vibração".to_string());
            }
            Ok(())
        })
    }
}

/// Vibra com um padrão em milissegundos, no formato do Android: o primeiro número é a espera
/// antes de começar e os seguintes alternam "vibra" e "para" (por exemplo `[0, 28, 55, 35]`).
/// Usa `Vibrator.vibrate(long[], int)` (com repetição `-1`, ou seja, toca uma vez só), que é
/// antigo mas existe em todas as versões. Um erro da ponte vira `Err`; nunca derruba o app.
pub fn vibrar_padrao(padrao_ms: &[u32]) -> Result<(), String> {
    if padrao_ms.is_empty() {
        return Ok(());
    }
    unsafe {
        let env = attach_jni_env();
        com_quadro(env, || {
            let vibrador = vibrador_do_aparelho(env)?;
            // O Java quer uma lista de `long` (64 bits); copiamos os tempos para uma lista dessas.
            let tempos: Vec<ndk_sys::jlong> = padrao_ms.iter().map(|&ms| ndk_sys::jlong::from(ms)).collect();
            let tamanho = tempos.len() as ndk_sys::jsize;
            let lista = (**env).NewLongArray.unwrap()(env, tamanho);
            if limpar_excecao(env) || lista.is_null() {
                return Err("o Android não criou a lista de tempos da vibração".to_string());
            }
            (**env).SetLongArrayRegion.unwrap()(env, lista, 0, tamanho, tempos.as_ptr());
            if limpar_excecao(env) {
                return Err("o Android não aceitou a lista de tempos da vibração".to_string());
            }
            call_void_method!(env, vibrador, "vibrate", "([JI)V", lista, -1_i32);
            if limpar_excecao(env) {
                return Err("o Android recusou a vibração".to_string());
            }
            Ok(())
        })
    }
}
