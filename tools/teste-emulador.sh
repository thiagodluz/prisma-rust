#!/usr/bin/env bash
# Abre o APK num celular virtual (emulador Android) e confere o teste de vida do M0b:
#   1. o app abre e continua aberto;
#   2. o contador de aberturas grava em arquivo (1 na primeira abertura, 2 na segunda);
#   3. o botão "Tocar som" responde (PRISMA_M0B som=ok);
#   4. o botão "Vibrar" responde (PRISMA_M0B vibracao=ok; o emulador pode não ter vibrador).
# Guarda o registro de erros (logcat) e fotos da tela numa pasta.
#
# Uso: tools/teste-emulador.sh <pasta-com-o-apk> <pasta-de-saida>
# Roda no GitHub (.github/workflows/android.yml), com o emulador já ligado, a partir da
# raiz do projeto. Termina com erro (1) se algo falhar.
#
# O app escreve no registro do Android linhas que começam com "PRISMA_M0B" (veja
# `rodar_diagnostico` em game/src/main.rs); este script lê essas linhas.

# Identificador do app: lido do game/Cargo.toml (rode o script da raiz do projeto).
PACOTE="$(sed -n 's/^package_name = "\(.*\)"/\1/p' game/Cargo.toml)"
if [ -z "$PACOTE" ]; then
  echo "Não achei package_name em game/Cargo.toml (rode o script da raiz do projeto)."
  exit 1
fi
echo "Pacote: $PACOTE"
PASTA_APK="${1:?faltou a pasta com o APK}"
SAIDA="${2:?faltou a pasta de saída}"

mkdir -p "$SAIDA"
APK="$(ls "$PASTA_APK"/*.apk | head -n 1)"
echo "APK: $APK"

# Mostra as linhas do registro que mais ajudam a achar um erro.
resumo() {
  echo "===== logcat (resumo) ====="
  grep -iE "PRISMA_M0B|FATAL|AndroidRuntime|panicked|SIGSEGV|SIGABRT|Abort message|UnsatisfiedLink|dlopen|libprisma|miniquad|DEBUG  " \
    "$SAIDA/logcat.txt" | head -150
  echo "===== fim do resumo ====="
}

# Guarda o registro atual, mostra o resumo e termina com erro.
falhou() {
  echo "ERRO: $1"
  adb logcat -d > "$SAIDA/logcat.txt"
  resumo
  exit 1
}

# Imprime uma foto reduzida em texto (base64) para poder ser lida direto no registro do
# GitHub, sem baixar o artefato. Uso: imprimir_foto <nome> <arquivo.png>
imprimir_foto() {
  if command -v convert > /dev/null; then
    convert "$2" -resize 270x -colors 48 "PNG8:$SAIDA/pequena-$1.png"
    echo "TELA_BASE64_INICIO $1"
    base64 -w0 "$SAIDA/pequena-$1.png"
    echo
    echo "TELA_BASE64_FIM"
  fi
}

# Abre o app como se fosse pelo ícone.
abrir() {
  adb shell monkey -p "$PACOTE" -c android.intent.category.LAUNCHER 1 > "$SAIDA/abertura.txt" 2>&1
  cat "$SAIDA/abertura.txt"
}

# Procura o centro de um botão no registro (o app escreve de novo a cada mudança de
# tamanho da tela; vale a última linha). Devolve "x y".
centro_do_botao() {
  grep -F "PRISMA_M0B botao=$1 " "$SAIDA/logcat.txt" | tail -n 1 | sed -n 's/.*x=\([0-9]*\) y=\([0-9]*\).*/\1 \2/p'
}

# Toca num botão e espera o app responder.
tocar() {
  local centro
  centro="$(centro_do_botao "$1")"
  [ -n "$centro" ] || falhou "o registro não tem a posição do botão '$1'"
  echo "Tocando em '$1' ($centro)"
  # shellcheck disable=SC2086
  adb shell input tap $centro
  sleep 3
  adb logcat -d > "$SAIDA/logcat.txt"
}

# Versão do Android e processadores do celular virtual (ajuda a ler o resultado).
adb shell getprop ro.build.version.release
adb shell getprop ro.product.cpu.abilist

if ! adb install -r "$APK" > "$SAIDA/instalacao.txt" 2>&1; then
  echo "A instalação FALHOU:"
  cat "$SAIDA/instalacao.txt"
  exit 1
fi
cat "$SAIDA/instalacao.txt"

# ---- 1. Primeira abertura ----
adb logcat -c
abrir
sleep 20
adb logcat -d > "$SAIDA/logcat.txt"
adb exec-out screencap -p > "$SAIDA/tela-1-abertura.png"
adb shell pidof "$PACOTE" > /dev/null || falhou "o app FECHOU sozinho depois de abrir."
grep -qF "PRISMA_M0B aberturas=1" "$SAIDA/logcat.txt" \
  || falhou "faltou a linha 'PRISMA_M0B aberturas=1' (o contador não gravou em arquivo?)."
echo "OK: abriu, e o contador marcou 1."
# O som precisa ter carregado (é aqui que um áudio quebrado apareceria).
grep -qF "PRISMA_M0B som_carregado=ok" "$SAIDA/logcat.txt" \
  || falhou "faltou a linha 'PRISMA_M0B som_carregado=ok' (o som não carregou)."
echo "OK: o som carregou."

# ---- 2. Toques nos botões ----
tocar som
grep -qF "PRISMA_M0B som=ok" "$SAIDA/logcat.txt" || falhou "faltou 'PRISMA_M0B som=ok' depois de tocar em 'Tocar som'."
echo "OK: o botão de som respondeu."

tocar vibrar
# O celular virtual pode não ter vibrador; o app então diz isso em vez de dar erro estranho.
if grep -qF "PRISMA_M0B vibracao=ok" "$SAIDA/logcat.txt"; then
  echo "OK: o botão de vibrar respondeu (vibrou)."
elif grep -qF "PRISMA_M0B vibracao=erro: este aparelho" "$SAIDA/logcat.txt"; then
  echo "OK: o botão de vibrar respondeu (o celular virtual não tem vibrador)."
else
  falhou "a linha 'PRISMA_M0B vibracao=' não veio, ou veio com um erro inesperado."
fi
adb shell pidof "$PACOTE" > /dev/null || falhou "o app FECHOU sozinho depois dos toques."
adb exec-out screencap -p > "$SAIDA/tela-2-depois-dos-toques.png"

# ---- 3. Segunda abertura (o arquivo do contador sobreviveu?) ----
adb shell am force-stop "$PACOTE"
adb logcat -c
abrir
sleep 10
adb logcat -d > "$SAIDA/logcat.txt"
adb shell pidof "$PACOTE" > /dev/null || falhou "o app FECHOU sozinho na segunda abertura."
grep -qF "PRISMA_M0B aberturas=2" "$SAIDA/logcat.txt" \
  || falhou "faltou a linha 'PRISMA_M0B aberturas=2' (o arquivo do contador não sobreviveu ao fechar o app?)."
echo "OK: na segunda abertura o contador marcou 2."
adb exec-out screencap -p > "$SAIDA/tela-3-segunda-abertura.png"

resumo
echo "OK: o teste de vida do M0b passou no celular virtual."
imprimir_foto depois-dos-toques "$SAIDA/tela-2-depois-dos-toques.png"
imprimir_foto segunda-abertura "$SAIDA/tela-3-segunda-abertura.png"
