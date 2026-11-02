"""Arquivos das pastas de exemplo do manabox (embutidos no programa por src-tauri/src/examples.rs).

Gera em src-tauri/assets/exemplo/: um cartaz PNG (o mesmo nas duas pastas, para o modo
Duplicados ter o que mostrar), dois PDFs e dois TXT. Rodar de novo só se o conteúdo mudar.
"""
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

OUT = Path(__file__).resolve().parent.parent / "src-tauri" / "assets" / "exemplo"
ICON = Path(__file__).resolve().parent.parent / "src-tauri" / "icons" / "128x128.png"

CONVENCAO = [
    "O manabox classifica os arquivos pelo nome.",
    "",
    "O nome é dividido em termos por hífen (-), sublinhado (_) ou espaço.",
    "Cada termo é procurado no índice (Editar índice): news, insta, promo...",
    "Sinônimos contam como o termo: newsletter = news, instagram = insta.",
    "",
    "Data de uso no nome: AAMMDD (241129 = 29/11/2024) ou AAMM (2412 = dez/2024).",
    "Sem data no nome, vale a data de modificação do arquivo.",
    "",
    "Estes são arquivos de exemplo (.example antes da extensão).",
    "Quando não precisar mais, apague a pasta manabox-exemplo e",
    "remova as pastas de exemplo em Editar pastas.",
]


def poster(path: Path):
    img = Image.new("RGB", (480, 270), "#2f6f62")
    draw = ImageDraw.Draw(img)
    icon = Image.open(ICON).convert("RGBA")
    img.paste(icon, (32, 71), icon)
    big = ImageFont.load_default(size=40)
    small = ImageFont.load_default(size=20)
    draw.text((184, 92), "PROMO", font=big, fill="#f6d55c")
    draw.text((184, 142), "black friday · exemplo", font=small, fill="#ffffff")
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


OUT.mkdir(parents=True, exist_ok=True)
poster(OUT / "cartaz.png")
pdf(OUT / "lancamento-newsletter.pdf", "Newsletter de lançamento (exemplo)", CONVENCAO)
pdf(OUT / "lancamento-instagram.pdf", "Post de lançamento no Instagram (exemplo)", CONVENCAO)
(OUT / "mala-direta.txt").write_text("Mala direta da promoção de dezembro (exemplo)\n\n" + "\n".join(CONVENCAO) + "\n")
(OUT / "email.txt").write_text("E-mail de lançamento (exemplo)\n\n" + "\n".join(CONVENCAO) + "\n")
print("\n".join(f"{p.name}: {p.stat().st_size} bytes" for p in sorted(OUT.iterdir())))
