#!/usr/bin/env python3
"""Generate a branded 1200x630 OG / social card from a title (no source image).

A dark gradient background, an accent bar, the wrapped title, an optional
subtitle, and a brand wordmark. Pure Pillow — runs anywhere. Mirrors the dynamic
Next.js `/og` route as a static, portable generator.

Examples
  og_card.py --title "Why your agent's memory returned nothing"
  og_card.py --title "Chronis 0.8.0" --subtitle "Web viewer + live demos" --brand AllSource
  og_card.py --title "..." --accent "#38bdf8" -o public/assets/blog/post-og.png
"""
import argparse
import sys
from pathlib import Path

try:
    from PIL import Image, ImageDraw, ImageFont
except ImportError:
    sys.exit("Pillow is required. Install it with:  python3 -m pip install pillow")

W, H = 1200, 630
MARGIN = 80


def hex_rgb(s: str) -> tuple[int, int, int]:
    s = s.lstrip("#")
    return tuple(int(s[i : i + 2], 16) for i in (0, 2, 4))  # type: ignore[return-value]


def vertical_gradient(size, top, bottom):
    img = Image.new("RGB", size, top)
    draw = ImageDraw.Draw(img)
    h = size[1]
    for y in range(h):
        t = y / h
        c = tuple(round(top[i] * (1 - t) + bottom[i] * t) for i in range(3))
        draw.line([(0, y), (size[0], y)], fill=c)
    return img


def load_font(size: int, bold: bool = True):
    candidates = (
        ["Inter-SemiBold.ttf", "Inter-Bold.ttf", "Arial Bold.ttf", "Helvetica-Bold.ttf",
         "DejaVuSans-Bold.ttf", "/System/Library/Fonts/Supplemental/Arial Bold.ttf"]
        if bold
        else ["Inter-Regular.ttf", "Arial.ttf", "Helvetica.ttf", "DejaVuSans.ttf",
              "/System/Library/Fonts/Supplemental/Arial.ttf"]
    )
    for name in candidates:
        try:
            return ImageFont.truetype(name, size)
        except OSError:
            continue
    return ImageFont.load_default()


def wrap(draw, text, font, max_w):
    words, lines, cur = text.split(), [], ""
    for word in words:
        test = f"{cur} {word}".strip()
        if draw.textlength(test, font=font) <= max_w:
            cur = test
        else:
            if cur:
                lines.append(cur)
            cur = word
    if cur:
        lines.append(cur)
    return lines


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--title", required=True)
    ap.add_argument("--subtitle", default="")
    ap.add_argument("--brand", default="")
    ap.add_argument("--accent", default="#38bdf8")
    ap.add_argument("-o", "--out", default="og-card.png")
    ap.add_argument("--bg-top", default="#0f172a", help="gradient top color")
    ap.add_argument("--bg-bottom", default="#020617", help="gradient bottom color")
    args = ap.parse_args()

    accent = hex_rgb(args.accent)
    img = vertical_gradient((W, H), hex_rgb(args.bg_top), hex_rgb(args.bg_bottom))
    draw = ImageDraw.Draw(img)

    # accent bar
    draw.rounded_rectangle([(MARGIN, 150), (MARGIN + 64, 158)], radius=4, fill=accent)

    # title (auto-shrink to fit up to 4 lines)
    size = 66
    while size >= 40:
        title_font = load_font(size)
        lines = wrap(draw, args.title, title_font, W - 2 * MARGIN)
        if len(lines) <= 4:
            break
        size -= 6
    line_h = size + 14
    y = 188
    for ln in lines[:4]:
        draw.text((MARGIN, y), ln, font=title_font, fill=(241, 245, 249))
        y += line_h

    if args.subtitle:
        draw.text((MARGIN, y + 8), args.subtitle, font=load_font(30, bold=False), fill=(148, 163, 184))

    if args.brand:
        draw.text((MARGIN, H - 72), args.brand, font=load_font(28), fill=accent)

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    img.save(out)
    kb = out.stat().st_size // 1024
    print(f"wrote {out}  ({W}x{H}, {kb} KB)")


if __name__ == "__main__":
    main()
