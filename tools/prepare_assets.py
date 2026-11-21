#!/usr/bin/env python3
"""Prepara as imagens do Prisma em Rust a partir das imagens do Prisma em JS.

Uso:
    python3 tools/prepare_assets.py <pasta-do-prisma-js> game

O que faz:
  1. Lê os atlas `gem-atlas.webp`, `burst-atlas.webp` e `cross-atlas.webp` (1254x1254, grade
     3x3 com sete pedras) e o `spectrum-gem.webp`.
  2. Recorta cada pedra pelo contorno dela (o conjunto de pixels ligados), e não pela grade:
     no atlas a ponta da pedra 6 invade 3 pixels a casa da pedra 3, e um corte reto da grade
     deixaria um pedacinho de uma pedra dentro da outra.
  3. Reduz com a escala que o JS usa em cada pedra (`--gem-scale` do style.css) e centraliza
     pelo pixel visível (transparência acima de 50%).
  4. Grava PNG 160x160 em `<jogo>/assets/`. A casa do tabuleiro equivale a 128 px dentro dele;
     os 16 px de cada lado são margem, para a pedra ampliada não ser cortada.
  5. Gera os ícones do Android a partir de `icon-512.png`.

Precisa de: pip install pillow numpy scipy   (só para preparar imagens; o jogo não usa Python).
"""
import sys
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

CELULA_ATLAS = 418   # cada casa da grade 3x3 do atlas (1254 / 3)
CELULA_PX = 128      # tamanho da casa do tabuleiro dentro do PNG
TELA_PX = 160        # tamanho do PNG: a casa mais a margem (160 / 128 = 1,25)
MIN_PEDRA = 2000     # componente com menos pixels que isto é poeira, não pedra

# Posição de cada pedra na grade do atlas, (coluna, linha), como no style.css do JS.
POSICAO = {0: (0, 0), 1: (1, 0), 2: (2, 0), 3: (0, 1), 4: (1, 1), 5: (2, 1), 6: (0, 2)}
# Escala de cada pedra (`--gem-scale` do style.css). Pulso e Raio usam a mesma da pedra de
# mesmo número.
ESCALA = [0.98, 1.0, 1.08, 0.87, 0.96, 1.08, 0.90]
# O Espectro aparece a 118% da casa no JS (`background-size: 118%`).
ESCALA_ESPECTRO = 1.18

ICONES_ANDROID = {"mdpi": 48, "hdpi": 72, "xhdpi": 96, "xxhdpi": 144, "xxxhdpi": 192}


def centraliza(pixels: np.ndarray, fator: float) -> Image.Image:
    """Reduz a pedra por `fator` e a põe no meio de uma tela transparente de 160x160."""
    imagem = Image.fromarray(pixels, "RGBA")
    novo = (max(1, round(imagem.width * fator)), max(1, round(imagem.height * fator)))
    # Reduzir em alfa pré-multiplicado ("RGBa"); senão as bordas transparentes escurecem.
    imagem = imagem.convert("RGBa").resize(novo, Image.LANCZOS).convert("RGBA")

    visivel = np.array(imagem)[:, :, 3] >= 128          # alfa de 50% em diante
    ys, xs = np.where(visivel)
    x0, x1, y0, y1 = xs.min(), xs.max() + 1, ys.min(), ys.max() + 1
    # Põe o centro da caixa visível no centro da tela (80, 80).
    dx = round(TELA_PX / 2 - (x0 + x1) / 2)
    dy = round(TELA_PX / 2 - (y0 + y1) / 2)
    tela = Image.new("RGBA", (TELA_PX, TELA_PX), (0, 0, 0, 0))
    tela.paste(imagem, (dx, dy))   # sem máscara: copia o alfa como está
    return tela


def pedras_do_atlas(caminho: Path) -> dict:
    """Devolve {índice da pedra: pixels RGBA recortados pelo contorno} para um atlas 3x3."""
    pixels = np.array(Image.open(caminho).convert("RGBA"))
    ligados, quantos = ndimage.label(pixels[:, :, 3] > 0)
    tamanhos = ndimage.sum(pixels[:, :, 3] > 0, ligados, range(1, quantos + 1))
    caixas = ndimage.find_objects(ligados)
    por_posicao = {pos: indice for indice, pos in POSICAO.items()}
    achadas = {}
    for numero, tamanho in enumerate(tamanhos, start=1):
        if tamanho < MIN_PEDRA:
            continue
        ys, xs = caixas[numero - 1]
        coluna = int((xs.start + xs.stop) / 2 // CELULA_ATLAS)
        linha = int((ys.start + ys.stop) / 2 // CELULA_ATLAS)
        indice = por_posicao[(coluna, linha)]
        recorte = pixels[ys, xs].copy()
        recorte[ligados[ys, xs] != numero] = 0     # apaga o que for de outra pedra
        achadas[indice] = recorte
    if sorted(achadas) != list(range(7)):
        sys.exit(f"{caminho.name}: esperava 7 pedras, achei as de índice {sorted(achadas)}")
    return achadas


def main() -> None:
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    origem, jogo = Path(sys.argv[1]), Path(sys.argv[2])
    saida = jogo / "assets"
    saida.mkdir(parents=True, exist_ok=True)

    for prefixo, atlas in (("gem", "gem-atlas.webp"), ("burst", "burst-atlas.webp"), ("cross", "cross-atlas.webp")):
        pedras = pedras_do_atlas(origem / atlas)
        for indice in range(7):
            fator = CELULA_PX * ESCALA[indice] / CELULA_ATLAS
            arquivo = saida / f"{prefixo}-{indice}.png"
            centraliza(pedras[indice], fator).save(arquivo)
            print("gerado", arquivo)

    espectro = np.array(Image.open(origem / "spectrum-gem.webp").convert("RGBA"))
    fator = CELULA_PX * ESCALA_ESPECTRO / espectro.shape[1]
    arquivo = saida / "spectrum.png"
    centraliza(espectro, fator).save(arquivo)
    print("gerado", arquivo)

    icone = Image.open(origem / "icon-512.png").convert("RGBA")
    for densidade, lado in ICONES_ANDROID.items():
        pasta = jogo / "android_res" / f"mipmap-{densidade}"
        pasta.mkdir(parents=True, exist_ok=True)
        icone.resize((lado, lado), Image.LANCZOS).save(pasta / "ic_launcher.png")
        print("gerado", pasta / "ic_launcher.png")


if __name__ == "__main__":
    main()
