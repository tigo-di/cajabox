"""Ícone do cajabox: baú em pixel art (16×16) com musgo e fecho dourado.

Gera src-tauri/icons/icon.svg (fonte) e os PNGs com pixels nítidos (ampliação sem suavizar).
icon.ico e icon.icns vêm de `pnpm tauri icon src-tauri/icons/icon.png`; rode este script
depois dele para regravar os PNGs nítidos.
"""
from pathlib import Path

from PIL import Image

GRID = """
................
.OOOOOOOOOOOOOO.
.OHHHHHHHHHHHHO.
.OLLLLLLLLLLLLO.
.OLMLLLMLLLMLLO.
.OMMMMMMMMMMMMO.
.OOOOOkkkkOOOOO.
.ODDDDkSSkDDDDO.
.gMMMMksskMMMMO.
.GgMMMkkkkMMLMO.
.OGMMMMMMMMMMMO.
.GgLMLMMMMLMMLO.
.gGgMMMMMMMMMMO.
.dGGgDDDDDDDDDO.
.dGdGgOOOOOOOOO.
................
""".split()
PAL = {"O":"#3b2412","D":"#6b4220","M":"#8f5a2b","L":"#b07a3e","H":"#c9934f",
       "G":"#4f7a28","g":"#78a83c","d":"#33521a","k":"#5a3d0c","s":"#c9952a","S":"#f6d55c"}
assert all(len(r)==16 for r in GRID) and len(GRID)==16, [len(r) for r in GRID]
OUT = Path(__file__).resolve().parent.parent / "src-tauri" / "icons"

rects = [f'<rect x="{x}" y="{y}" width="1" height="1" fill="{PAL[c]}"/>' for y, r in enumerate(GRID) for x, c in enumerate(r) if c != "."]
(OUT / "icon.svg").write_text(
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" width="1024" height="1024" shape-rendering="crispEdges">\n'
    + "\n".join(rects) + "\n</svg>\n"
)

img = Image.new("RGBA", (16, 16), (0, 0, 0, 0))
for y, r in enumerate(GRID):
    for x, c in enumerate(r):
        if c != ".":
            img.putpixel((x, y), tuple(int(PAL[c][i:i + 2], 16) for i in (1, 3, 5)) + (255,))
for name, size in [("32x32.png", 32), ("64x64.png", 64), ("128x128.png", 128), ("128x128@2x.png", 256), ("icon.png", 1024)]:
    img.resize((size, size), Image.NEAREST).save(OUT / name)
