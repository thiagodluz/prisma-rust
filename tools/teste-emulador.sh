#!/usr/bin/env bash
# Abre o APK num celular virtual (emulador Android) e JOGA de verdade:
#   1. o app abre direto no jogo (Zen) e continua aberto;
#   2. Zen: 5 vezes, toca em "Dica", arrasta o dedo de uma pedra da dica até a outra, espera
#      a animação terminar e confere que a jogada valeu e os pontos subiram;
#   3. Clássico: troca de aba e faz 5 jogadas tocando numa pedra e depois na vizinha;
#   4. Novo jogo (com a confirmação) no Clássico zera os pontos;
#   5. volta ao Zen e confere que os pontos do Zen continuam lá.
# Guarda o registro de erros (logcat) e uma foto da tela de cada etapa numa pasta.
#
# Uso: tools/teste-emulador.sh <pasta-com-o-apk> <pasta-de-saida>
# Roda no GitHub (.github/workflows/android.yml), com o emulador já ligado, a partir da
# raiz do projeto. Termina com erro (1) se algo falhar.
#
# O app escreve no registro do Android linhas que começam com "PRISMA_M1" (veja `App::registro`
# em game/src/app.rs e `rodar_jogo` em game/src/main.rs): a posição do tabuleiro e dos botões
# (em pixels reais, medidos a partir do canto da janela do app), cada toque que recebe e o que
# acontece no jogo. Este script lê essas linhas.

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
  grep -iE "PRISMA_M1|FATAL|AndroidRuntime|panicked|SIGSEGV|SIGABRT|Abort message|UnsatisfiedLink|dlopen|libprisma|miniquad|DEBUG  " \
    "$SAIDA/logcat.txt" | head -200
  echo "===== fim do resumo ====="
}

# Guarda o registro atual, mostra o resumo e termina com erro.
falhou() {
  echo "ERRO: $1"
  adb logcat -d > "$SAIDA/logcat.txt"
  adb exec-out screencap -p > "$SAIDA/tela-do-erro.png" 2> /dev/null
  resumo
  imprimir_foto do-erro "$SAIDA/tela-do-erro.png"
  exit 1
}

# Imprime uma foto reduzida em texto (base64) para poder ser lida direto no registro do
# GitHub, sem baixar o artefato. Uso: imprimir_foto <nome> <arquivo.png>
imprimir_foto() {
  if command -v convert > /dev/null && [ -s "$2" ]; then
    convert "$2" -resize 270x -colors 48 "PNG8:$SAIDA/pequena-$1.png"
    echo "TELA_BASE64_INICIO $1"
    base64 -w0 "$SAIDA/pequena-$1.png"
    echo
    echo "TELA_BASE64_FIM"
  fi
}

# Tira a foto da tela, guarda e imprime reduzida. Uso: foto <nome>
foto() {
  adb exec-out screencap -p > "$SAIDA/tela-$1.png"
  imprimir_foto "$1" "$SAIDA/tela-$1.png"
}

# Abre o app como se fosse pelo ícone.
abrir() {
  adb shell monkey -p "$PACOTE" -c android.intent.category.LAUNCHER 1 > "$SAIDA/abertura.txt" 2>&1
  cat "$SAIDA/abertura.txt"
}

# Lê o registro de novo (guardado em $SAIDA/logcat.txt).
ler_registro() {
  adb logcat -d > "$SAIDA/logcat.txt"
}

# Confere que o app continua aberto. Uso: continua_aberto <o que acabou de acontecer>
continua_aberto() {
  adb shell pidof "$PACOTE" > /dev/null || falhou "o app FECHOU sozinho $1."
}

# Quantas vezes uma linha do registro aparece. Uso: contar <texto>
contar() {
  grep -cF "$1" "$SAIDA/logcat.txt"
}

# Procura o centro de um botão no registro (o app escreve de novo a cada mudança de tamanho
# da tela ou de modo; vale a última linha). Devolve "x y".
centro_do_botao() {
  grep -F "PRISMA_M1 botao=$1 " "$SAIDA/logcat.txt" | tail -n 1 | sed -n 's/.*x=\([0-9]*\) y=\([0-9]*\).*/\1 \2/p'
}

# Toca em "Dica" e devolve "a b" (as duas casas da dica). Exige uma linha NOVA no registro,
# para nunca usar a dica de uma rodada anterior caso o toque não tenha pegado.
pedir_dica() {
  local antes linha
  antes="$(contar "PRISMA_M1 dica ")"
  tocar_botao dica
  [ "$(contar "PRISMA_M1 dica ")" -gt "$antes" ] || falhou "o toque em 'Dica' não gerou uma dica nova"
  linha="$(grep -F "PRISMA_M1 dica " "$SAIDA/logcat.txt" | tail -n 1)"
  DICA_A="$(echo "$linha" | sed -n 's/.* a=\([0-9]*\) b=.*/\1/p')"
  DICA_B="$(echo "$linha" | sed -n 's/.* b=\([0-9]*\).*/\1/p')"
  [ -n "$DICA_A" ] && [ -n "$DICA_B" ] || falhou "a linha da dica veio sem as duas casas: $linha"
}

# Toca num botão e espera um pouco o app responder.
tocar_botao() {
  local centro
  centro="$(centro_do_botao "$1")"
  [ -n "$centro" ] || falhou "o registro não tem a posição do botão '$1'"
  # shellcheck disable=SC2086
  set -- "$1" $centro
  echo "Tocando em '$1' (app: $2 $3; tela: $(($2 + OFF_X)) $(($3 + OFF_Y)))"
  adb shell input tap $(($2 + OFF_X)) $(($3 + OFF_Y))
  sleep 1
  ler_registro
}

# Posição do tabuleiro (x y lado), da última linha do registro.
TX=0
TY=0
LADO=0
ler_tabuleiro() {
  local linha
  linha="$(grep -F "PRISMA_M1 tabuleiro " "$SAIDA/logcat.txt" | tail -n 1)"
  TX="$(echo "$linha" | sed -n 's/.* x=\([0-9]*\) .*/\1/p')"
  TY="$(echo "$linha" | sed -n 's/.* y=\([0-9]*\) .*/\1/p')"
  LADO="$(echo "$linha" | sed -n 's/.*lado=\([0-9]*\).*/\1/p')"
  [ -n "$TX" ] && [ -n "$TY" ] && [ -n "$LADO" ] && [ "$LADO" -gt 0 ] \
    || falhou "o registro não tem a posição do tabuleiro"
}

# Centro de uma casa do tabuleiro (índice 0..63, linha a linha), em coordenadas da TELA (já
# com o deslocamento da janela). Devolve "x y".
# x = tx + (coluna + 0,5) * lado / 8; y = ty + (linha + 0,5) * lado / 8
centro_da_casa() {
  local linha=$(($1 / 8)) coluna=$(($1 % 8))
  echo $((TX + OFF_X + (2 * coluna + 1) * LADO / 16)) $((TY + OFF_Y + (2 * linha + 1) * LADO / 16))
}

# O app mede os toques a partir do canto da PRÓPRIA janela, mas o `adb input` usa coordenadas
# da TELA; no emulador a janela começa abaixo da barra de status (uns 28 px). Para descobrir o
# deslocamento sem depender da versão do Android, toca num ponto da tela que não faz nada no
# jogo (a margem de cima, acima das abas) e compara com a linha "PRISMA_M1 toque" que o app
# escreve. Preenche OFF_X e OFF_Y.
OFF_X=0
OFF_Y=0
calibrar_toque() {
  local antes tela_x tela_y linha app_x app_y
  antes="$(contar "PRISMA_M1 toque ")"
  tela_x=$((TX + LADO / 2))
  tela_y=60
  echo "Calibrando: tocando na margem de cima, na tela em ($tela_x,$tela_y)"
  adb shell input tap "$tela_x" "$tela_y"
  sleep 1
  ler_registro
  [ "$(contar "PRISMA_M1 toque ")" -gt "$antes" ] || falhou "o app não registrou o toque de calibração"
  linha="$(grep -F "PRISMA_M1 toque " "$SAIDA/logcat.txt" | tail -n 1)"
  app_x="$(echo "$linha" | sed -n 's/.* x=\([0-9]*\) y=.*/\1/p')"
  app_y="$(echo "$linha" | sed -n 's/.* y=\([0-9]*\).*/\1/p')"
  [ -n "$app_x" ] && [ -n "$app_y" ] || falhou "a linha do toque veio sem x e y: $linha"
  OFF_X=$((tela_x - app_x))
  OFF_Y=$((tela_y - app_y))
  echo "O app viu o toque em ($app_x,$app_y): a janela está deslocada em ($OFF_X,$OFF_Y) px da tela"
  # Um deslocamento absurdo quer dizer que o toque caiu em outro lugar.
  [ "$OFF_X" -ge 0 ] && [ "$OFF_X" -le 100 ] && [ "$OFF_Y" -ge 0 ] && [ "$OFF_Y" -le 200 ] \
    || falhou "deslocamento da janela fora do esperado: ($OFF_X,$OFF_Y)"
}

# Espera (até 30 s, de meio em meio segundo) o registro ter mais linhas "animacao_terminou"
# do que antes. Uso: esperar_animacao <quantas havia antes>
esperar_animacao() {
  local antes="$1" i
  for i in $(seq 1 60); do
    sleep 0.5
    ler_registro
    if [ "$(contar "PRISMA_M1 animacao_terminou")" -gt "$antes" ]; then
      return 0
    fi
  done
  falhou "a animação não terminou em 30 s"
}

# Pontos da última linha "jogada" do registro.
ultimos_pontos() {
  grep -F "PRISMA_M1 jogada " "$SAIDA/logcat.txt" | tail -n 1 | sed -n 's/.*pontos=\([0-9]*\).*/\1/p'
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

# ---- 1. Abertura: o app abre direto no jogo ----
adb logcat -c
abrir
sleep 15
ler_registro
continua_aberto "depois de abrir"
grep -qF "PRISMA_M1 pronto modo=zen" "$SAIDA/logcat.txt" \
  || falhou "faltou a linha 'PRISMA_M1 pronto modo=zen' (o jogo não abriu?)."
echo "OK: o jogo abriu no modo Zen."
ler_tabuleiro
echo "Tabuleiro (coordenadas do app): x=$TX y=$TY lado=$LADO"
for botao in zen classico embaralhar dica novo_jogo confirmar; do
  [ -n "$(centro_do_botao "$botao")" ] || falhou "o registro não tem a posição do botão '$botao'"
done
foto 1-zen-inicio
calibrar_toque

# ---- 2. Zen: 5 jogadas arrastando o dedo ----
pontos_zen=0
for rodada in 1 2 3 4 5; do
  pedir_dica
  a="$DICA_A"
  b="$DICA_B"
  echo "Rodada $rodada (Zen): a dica pede trocar a casa $a com a $b"
  [ "$rodada" = 1 ] && foto 2-zen-dica
  # Não precisa esperar a dica sumir: o arrasto abaixo apaga a dica sozinho.
  animacoes="$(contar "PRISMA_M1 animacao_terminou")"
  # shellcheck disable=SC2046
  set -- $(centro_da_casa "$a") $(centro_da_casa "$b")
  echo "Arrastando de ($1,$2) até ($3,$4)"
  adb shell input swipe "$1" "$2" "$3" "$4" 200
  esperar_animacao "$animacoes"
  continua_aberto "durante a jogada $rodada do Zen"
  linha_jogada="$(grep -F "PRISMA_M1 jogada " "$SAIDA/logcat.txt" | tail -n 1)"
  echo "$linha_jogada"
  echo "$linha_jogada" | grep -qF "valida=true" || falhou "a jogada $rodada do Zen não foi válida: $linha_jogada"
  novos="$(ultimos_pontos)"
  [ -n "$novos" ] && [ "$novos" -gt "$pontos_zen" ] \
    || falhou "os pontos do Zen não subiram na jogada $rodada (antes $pontos_zen, agora '$novos')"
  pontos_zen="$novos"
  [ "$rodada" = 3 ] && foto 3-zen-depois-da-jogada
  sleep 1
done
echo "OK: 5 jogadas no Zen, pontos em $pontos_zen."
foto 4-zen-fim

# ---- 3. Clássico: 5 jogadas tocando numa pedra e depois na vizinha ----
tocar_botao classico
sleep 1
ler_registro
grep -qF "PRISMA_M1 modo=classico pontos=0" "$SAIDA/logcat.txt" \
  || falhou "faltou 'PRISMA_M1 modo=classico pontos=0' depois de tocar na aba Clássico"
echo "OK: mudou para o Clássico (começa com 0 pontos)."
ler_tabuleiro # o tabuleiro muda de lugar quando some o botão Embaralhar
pontos_classico=0
for rodada in 1 2 3 4 5; do
  pedir_dica
  a="$DICA_A"
  b="$DICA_B"
  echo "Rodada $rodada (Clássico): a dica pede trocar a casa $a com a $b"
  animacoes="$(contar "PRISMA_M1 animacao_terminou")"
  # shellcheck disable=SC2046
  set -- $(centro_da_casa "$a") $(centro_da_casa "$b")
  echo "Tocando em ($1,$2) e depois em ($3,$4)"
  adb shell input tap "$1" "$2"
  sleep 0.5
  adb shell input tap "$3" "$4"
  esperar_animacao "$animacoes"
  continua_aberto "durante a jogada $rodada do Clássico"
  linha_jogada="$(grep -F "PRISMA_M1 jogada " "$SAIDA/logcat.txt" | tail -n 1)"
  echo "$linha_jogada"
  echo "$linha_jogada" | grep -qF "valida=true" || falhou "a jogada $rodada do Clássico não foi válida: $linha_jogada"
  novos="$(ultimos_pontos)"
  [ -n "$novos" ] && [ "$novos" -gt "$pontos_classico" ] \
    || falhou "os pontos do Clássico não subiram na jogada $rodada (antes $pontos_classico, agora '$novos')"
  pontos_classico="$novos"
  sleep 1
done
echo "OK: 5 jogadas no Clássico, pontos em $pontos_classico."
foto 5-classico

# ---- 4. Novo jogo no Clássico (com a confirmação) ----
tocar_botao novo_jogo
foto 6-classico-confirmando
tocar_botao confirmar
grep -qF "PRISMA_M1 novo_jogo modo=classico pontos=0" "$SAIDA/logcat.txt" \
  || falhou "faltou 'PRISMA_M1 novo_jogo modo=classico pontos=0' depois de confirmar"
echo "OK: Novo jogo no Clássico zerou os pontos."
continua_aberto "depois do Novo jogo"

# ---- 5. Volta ao Zen: os pontos do Zen continuam ----
tocar_botao zen
sleep 1
ler_registro
ultima_troca="$(grep -F "PRISMA_M1 modo=zen " "$SAIDA/logcat.txt" | tail -n 1)"
echo "$ultima_troca"
echo "$ultima_troca" | grep -qF "pontos=$pontos_zen" \
  || falhou "ao voltar para o Zen os pontos não eram $pontos_zen: '$ultima_troca'"
echo "OK: de volta ao Zen, os $pontos_zen pontos continuam."
continua_aberto "ao voltar para o Zen"
foto 7-zen-de-volta

resumo
echo "OK: o jogo rodou no celular virtual (Zen e Clássico, toque e arrasto)."
