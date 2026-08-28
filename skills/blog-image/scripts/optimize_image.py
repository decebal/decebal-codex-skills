#!/usr/bin/env python3
"""Optimize a screenshot/image into a web-ready blog hero or OG/social card (WebP).

Presets crop+resize to an exact size (cover); `raw` keeps the aspect ratio and
only caps the width. Output defaults to WebP at quality 85 — the same recipe as
the proven `cwebp -q 85` workflow, but pure-Python (Pillow) so it runs anywhere,
including the Codex desktop app sandbox.

Examples
  optimize_image.py shot.png --preset og   -o public/assets/blog/my-post.webp
  optimize_image.py shot.png --preset hero
  optimize_image.py wide.png --preset raw  --max-width 1600
"""
import argparse
import sys
from pathlib import Path

try:
    from PIL import Image
except ImportError:
    sys.exit("Pillow is required. Install it with:  python3 -m pip install pillow")

# name -> (width, height). OG is the Open Graph / X (Twitter) card standard.
PRESETS = {
    "og": (1200, 630),
    "hero": (1600, 840),
    "thumb": (800, 420),
    "square": (1080, 1080),
}


def fit_cover(img: "Image.Image", size: tuple[int, int]) -> "Image.Image":
    """Resize then center-crop so the image exactly fills `size` (no distortion)."""
    tw, th = size
    w, h = img.size
    scale = max(tw / w, th / h)
    nw, nh = round(w * scale), round(h * scale)
    img = img.resize((nw, nh), Image.LANCZOS)
    left, top = (nw - tw) // 2, (nh - th) // 2
    return img.crop((left, top, left + tw, top + th))


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("input", help="source PNG/JPG/WebP")
    ap.add_argument(
        "--preset",
        choices=[*PRESETS, "raw"],
        default="og",
        help="og=1200x630 (default), hero=1600x840, thumb, square, or raw (keep aspect)",
    )
    ap.add_argument("-o", "--out", help="output path (default: <input>.webp)")
    ap.add_argument("-q", "--quality", type=int, default=85, help="WebP quality 0-100 (default 85)")
    ap.add_argument("--max-width", type=int, help="raw preset only: cap width, keep aspect")
    ap.add_argument("--format", choices=["webp", "png", "jpg"], default="webp")
    args = ap.parse_args()

    src = Path(args.input)
    if not src.is_file():
        sys.exit(f"no such file: {src}")

    img = Image.open(src).convert("RGB")

    if args.preset == "raw":
        if args.max_width and img.width > args.max_width:
            r = args.max_width / img.width
            img = img.resize((args.max_width, round(img.height * r)), Image.LANCZOS)
    else:
        img = fit_cover(img, PRESETS[args.preset])

    out = Path(args.out) if args.out else src.with_suffix(f".{args.format}")
    out.parent.mkdir(parents=True, exist_ok=True)

    if args.format == "webp":
        img.save(out, "WEBP", quality=args.quality, method=6)
    elif args.format == "jpg":
        img.save(out, "JPEG", quality=args.quality, optimize=True)
    else:
        img.save(out, "PNG", optimize=True)

    kb = out.stat().st_size // 1024
    print(f"wrote {out}  ({img.width}x{img.height}, {kb} KB)")


if __name__ == "__main__":
    main()
