"""Renders the installer bitmaps from the app icon and Inter.

    python scripts/installer_art.py

NSIS (MUI2) wants 24-bit BMPs: a 164x314 sidebar for the welcome/finish pages
and a 150x57 header. Both are drawn at 2x so they stay sharp on high-DPI
screens; NSIS scales them to fit.
"""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
ICON = Image.open(ROOT / "assets/logo/icon-512.png").convert("RGBA")
FONT_MEDIUM = str(ROOT / "assets/fonts/Inter-Medium.ttf")
FONT_REGULAR = str(ROOT / "assets/fonts/Inter-Regular.ttf")
OUT = ROOT / "assets/installer"

BG = (0x12, 0x13, 0x15)
TEXT = (0xE6, 0xE7, 0xE9)
DIM = (0x9A, 0x9E, 0xA6)
TEAL = (0x38, 0xB2, 0xA6)
INK = (0x12, 0x13, 0x15)


def icon(size):
    return ICON.resize((size, size), Image.LANCZOS)


def sidebar():
    w, h = 328, 628
    img = Image.new("RGBA", (w, h), BG + (255,))

    # Quiet current lines along the bottom, echoing the mark.
    lines = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    d = ImageDraw.Draw(lines)
    for i, alpha in enumerate([70, 48, 30]):
        r = 520 + i * 46
        box = (w / 2 - r, h - 120 + i * 18, w / 2 + r, h - 120 + i * 18 + 2 * r)
        d.arc(box, 200, 340, fill=TEAL + (alpha,), width=3)
    img.alpha_composite(lines)

    size = 132
    img.alpha_composite(icon(size), ((w - size) // 2, 150))
    d = ImageDraw.Draw(img)
    d.text((w / 2, 150 + size + 44), "Riptide", font=ImageFont.truetype(FONT_MEDIUM, 46), fill=TEXT, anchor="mm")
    d.text((w / 2, 150 + size + 92), "BitTorrent client", font=ImageFont.truetype(FONT_REGULAR, 22), fill=DIM, anchor="mm")
    img.convert("RGB").save(OUT / "sidebar.bmp")


def header():
    w, h = 300, 114
    img = Image.new("RGBA", (w, h), (255, 255, 255, 255))
    size = 60
    font = ImageFont.truetype(FONT_MEDIUM, 34)
    text_w = ImageDraw.Draw(img).textlength("Riptide", font=font)
    gap = 14
    total = size + gap + text_w
    x = w - 24 - total  # right-aligned, the header image sits on the right
    img.alpha_composite(icon(size), (int(x), (h - size) // 2))
    ImageDraw.Draw(img).text((x + size + gap, h / 2), "Riptide", font=font, fill=INK, anchor="lm")
    img.convert("RGB").save(OUT / "header.bmp")


if __name__ == "__main__":
    sidebar()
    header()
    print("wrote", OUT / "sidebar.bmp", OUT / "header.bmp")
