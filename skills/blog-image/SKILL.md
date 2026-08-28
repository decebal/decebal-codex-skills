---
name: blog-image
description: Turn a screenshot into a web-optimized blog hero or 1200x630 OG/social image (WebP), or generate a branded OG card from a title. Use when the user wants a blog hero image, social/OG share card, an optimized/resized image for the web, or to shrink a PNG to WebP.
---

Produce web-ready blog imagery: optimize a screenshot into a hero/OG WebP, or
generate a branded OG card from a title. Pure-Python (Pillow), so it runs locally
and in the Codex desktop app sandbox — no headless browser, no image API.

## When to use which script

- **Have a screenshot / image** → `scripts/optimize_image.py` (crop + resize + WebP).
- **Have only a title, need a share card** → `scripts/og_card.py` (branded 1200x630).

## Running the scripts

The scripts need Pillow. Pick whichever fits the environment:

- **uv (zero setup, recommended):** `uv run --with pillow python scripts/<script>.py ...`
- **plain python:** `python3 -m pip install pillow` once, then `python3 scripts/<script>.py ...`
- **Codex desktop app:** Pillow is already available — just run `python scripts/<script>.py ...`.

Run from the skill directory, or pass absolute paths to the scripts.

## optimize_image.py — screenshot → web image

```
optimize_image.py <input> [--preset og|hero|thumb|square|raw] [-o out] [-q 85] [--max-width N] [--format webp|png|jpg]
```

- `--preset og` (default) → **1200x630** (Open Graph / X card), center-cropped (cover).
- `--preset hero` → 1600x840 (larger blog hero, same ratio).
- `--preset raw` → keep aspect ratio; with `--max-width N` caps the width.
- Output defaults to `<input>.webp` at quality 85 (the proven `cwebp -q 85` recipe).

Examples:
```
optimize_image.py shot.png --preset og   -o public/assets/blog/my-post.webp
optimize_image.py shot.png --preset hero
optimize_image.py wide.png --preset raw  --max-width 1600
```

## og_card.py — title → branded share card

```
og_card.py --title "..." [--subtitle "..."] [--brand "..."] [--accent "#38bdf8"] [-o og-card.png] [--bg-top #0f172a] [--bg-bottom #020617]
```

- Renders a dark-gradient 1200x630 card: accent bar, auto-wrapped/auto-shrunk title (≤4 lines), optional subtitle, brand wordmark.
- Output is PNG; run it through `optimize_image.py --preset og` if you want WebP.

Example:
```
og_card.py --title "Chronis 0.8.0" --subtitle "Web viewer + live demos" --brand AllSource -o og.png
optimize_image.py og.png --preset og -o public/assets/blog/chronis-0-8-0-og.webp
```

## Workflow guidance (for the agent)

1. Decide intent from the user's request: optimize an existing image, or generate a card from a title.
2. For blog posts, write the final WebP to the site's blog asset dir (commonly
   `public/assets/blog/<slug>.webp`) and remind the user to set the post's
   `image:` frontmatter to that path.
3. Default to `--preset og` for share cards and `--preset hero` for in-post heros.
4. Report the output path, dimensions, and file size.

## Example output

`examples/sample-og.webp` is a rendered card (`og_card.py` → `optimize_image.py
--preset og`, 1200x630, 16 KB) — what a generated share card looks like.

## Notes

- WebP is written by Pillow directly — `cwebp`/ImageMagick are NOT required.
- For a screenshot of a *running page*, capture it first (your editor's screenshot
  tool, macOS CleanShot, or any browser screenshot), then feed the PNG here.
- Tested by `tests/test_blog_image.sh` (preset dimensions, WebP output, `--max-width`).
