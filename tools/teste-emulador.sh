#!/usr/bin/env bash
# Abre o APK num celular virtual (emulador Android) e confere se o app continua aberto.
# Guarda o registro de erros (logcat) e uma foto da tela numa pasta.
#
# Uso: tools/teste-emulador.sh <pasta-com-o-apk> <pasta-de-saida>
# Roda no GitHub (.github/workflows/android.yml), com o emulador já ligado.
# Termina com erro (1) se o app não instalar ou se fechar sozinho.

PACOTE="com.thiagodluz.prisma.rust"
PASTA_APK="${1:?faltou a pasta com o APK}"
SAIDA="${2:?faltou a pasta de saída}"

mkdir -p "$SAIDA"
APK="$(ls "$PASTA_APK"/*.apk | head -n 1)"
echo "APK: $APK"

# Versão do Android e processadores do celular virtual (ajuda a ler o resultado).
adb shell getprop ro.build.version.release
adb shell getprop ro.product.cpu.abilist

if ! adb install -r "$APK" > "$SAIDA/instalacao.txt" 2>&1; then
  echo "A instalação FALHOU:"
  cat "$SAIDA/instalacao.txt"
  exit 1
fi
cat "$SAIDA/instalacao.txt"

# Limpa o registro e abre o app como se fosse pelo ícone.
adb logcat -c
adb shell monkey -p "$PACOTE" -c android.intent.category.LAUNCHER 1 > "$SAIDA/abertura.txt" 2>&1
cat "$SAIDA/abertura.txt"

# Espera o app subir e tira a foto da tela.
sleep 20
adb exec-out screencap -p > "$SAIDA/tela.png"
adb logcat -d > "$SAIDA/logcat.txt"
adb shell dumpsys activity activities | grep -E "mResumedActivity|topResumedActivity" || true

# As linhas do registro que mais interessam para achar um erro.
echo "===== logcat (resumo) ====="
grep -iE "FATAL|AndroidRuntime|panicked|SIGSEGV|SIGABRT|Abort message|UnsatisfiedLink|dlopen|libprisma|prisma|miniquad|DEBUG  " \
  "$SAIDA/logcat.txt" | head -150
echo "===== fim do resumo ====="

if adb shell pidof "$PACOTE" > /dev/null; then
  echo "OK: o app continua aberto depois de 20 segundos."
  # Foto reduzida em texto (base64) para poder ser lida direto no registro do GitHub.
  if command -v convert > /dev/null; then
    convert "$SAIDA/tela.png" -resize 270x -colors 48 "PNG8:$SAIDA/tela-pequena.png"
    echo "TELA_BASE64_INICIO"
    base64 -w0 "$SAIDA/tela-pequena.png"
    echo
    echo "TELA_BASE64_FIM"
  fi
else
  echo "ERRO: o app FECHOU sozinho."
  exit 1
fi
