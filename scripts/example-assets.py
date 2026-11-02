"""Arquivos das pastas de exemplo do cajabox (embutidos no programa por src-tauri/src/examples.rs).

Gera em src-tauri/assets/examples/{pt,en}/: um cartaz PNG (o mesmo nas pastas, para o modo
Duplicados ter o que mostrar), dois PDFs e dois TXT. Rodar de novo só se o conteúdo mudar.
"""
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

OUT = Path(__file__).resolve().parent.parent / "src-tauri" / "assets" / "examples"
ICON = Path(__file__).resolve().parent.parent / "src-tauri" / "icons" / "128x128.png"

TEXTS = {
    "pt": {
        "convention": [
            "O cajabox classifica os arquivos pelo nome.",
            "",
            "O nome é dividido em termos por hífen (-), sublinhado (_) ou espaço.",
            "Cada termo é procurado no índice (Editar índice): news, insta, promo...",
            "Sinônimos contam como o termo: newsletter = news, instagram = insta.",
            "",
            "Data de uso no nome: AAMMDD (241129 = 29/11/2024) ou AAMM (2412 = dez/2024).",
            "Sem data no nome, vale a data de modificação do arquivo.",
            "",
            "Estes são arquivos de exemplo (.example antes da extensão).",
            "Quando não precisar mais, apague a pasta cajabox-exemplo e",
            "remova as pastas de exemplo em Editar pastas.",
        ],
        "poster": "black friday · exemplo",
        "files": {
            "lancamento-newsletter.pdf": "Newsletter de lançamento (exemplo)",
            "lancamento-instagram.pdf": "Post de lançamento no Instagram (exemplo)",
            "mala-direta.txt": "Mala direta da promoção de dezembro (exemplo)",
            "email.txt": "E-mail de lançamento (exemplo)",
        },
    },
    "en": {
        "convention": [
            "cajabox classifies files by their names.",
            "",
            "The name is split into terms by hyphen (-), underscore (_) or space.",
            "Each term is looked up in the index (Edit index): news, insta, promo...",
            "Synonyms count as the term: newsletter = news, instagram = insta.",
            "",
            "Date of use in the name: YYMMDD (241129 = 2024-11-29) or YYMM (2412 = 2024-12).",
            "Without a date in the name, the file's modification date is used.",
            "",
            "These are example files (.example before the extension).",
            "When you no longer need them, delete the cajabox-examples folder",
            "and remove the example folders under Folders > Edit.",
        ],
        "poster": "black friday · example",
        "files": {
            "launch-newsletter.pdf": "Launch newsletter (example)",
            "launch-instagram.pdf": "Instagram launch post (example)",
            "direct-mail.txt": "December promotion direct mail (example)",
            "email.txt": "Launch e-mail (example)",
        },
    },
}


def poster(path: Path, subtitle: str):
    img = Image.new("RGB", (480, 270), "#2f6f62")
    draw = ImageDraw.Draw(img)
    icon = Image.open(ICON).convert("RGBA")
    img.paste(icon, (32, 71), icon)
    big = ImageFont.load_default(size=40)
    small = ImageFont.load_default(size=20)
    draw.text((184, 92), "PROMO", font=big, fill="#f6d55c")
    draw.text((184, 142), subtitle, font=small, fill="#ffffff")
    img.save(path, optimize=True)


def pdf(path: Path, title: str, lines: list[str]):
    """PDF de uma página com texto (Helvetica, WinAnsi), sem dependências."""
    def esc(text: str) -> str:
        return text.replace("\\", "\\\\").replace("(", "\\(").replace(")", "\\)")

    ops = ["BT", "/F1 18 Tf", "56 780 Td", f"({esc(title)}) Tj", "/F1 11 Tf", "16 TL", "0 -10 Td"]
    for line in lines:
        ops += ["T*", f"({esc(line)}) Tj"]
    ops.append("ET")
    content = "\n".join(ops).encode("cp1252")
    objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
        b"<< /Length %d >>\nstream\n" % len(content) + content + b"\nendstream",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    ]
    out = bytearray(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n")
    offsets = []
    for number, body in enumerate(objects, 1):
        offsets.append(len(out))
        out += b"%d 0 obj\n" % number + body + b"\nendobj\n"
    xref = len(out)
    out += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objects) + 1)
    out += b"".join(b"%010d 00000 n \n" % o for o in offsets)
    out += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (len(objects) + 1, xref)
    path.write_bytes(bytes(out))


for lang, texts in TEXTS.items():
    out = OUT / lang
    out.mkdir(parents=True, exist_ok=True)
    poster(out / "poster.png", texts["poster"])
    for name, title in texts["files"].items():
        if name.endswith(".pdf"):
            pdf(out / name, title, texts["convention"])
        else:
            (out / name).write_text(title + "\n\n" + "\n".join(texts["convention"]) + "\n")
for path in sorted(OUT.rglob("*.*")):
    print(f"{path.relative_to(OUT)}: {path.stat().st_size} bytes")
