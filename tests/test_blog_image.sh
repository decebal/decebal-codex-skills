#!/usr/bin/env bash
# Tests the blog-image skill scripts. Dep-aware: needs Pillow (directly or via uv);
# skips (exit 0) if neither is available so CI without Python tooling stays green.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SK="$ROOT/skills/blog-image/scripts"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# Pick a Python that can import Pillow: a system python with PIL, else `uv` which
# provisions one on the fly.
if python3 -c "import PIL" >/dev/null 2>&1; then
  PY=(python3)
elif command -v uv >/dev/null 2>&1; then
  PY=(uv run --quiet --with pillow python3)
else
  echo "SKIP blog-image: Pillow not available (pip install pillow, or install uv)"
  exit 0
fi

fail() { echo "FAIL blog-image: $1" >&2; exit 1; }
dims() { "${PY[@]}" -c "from PIL import Image; print('%dx%d' % Image.open('$1').size)"; }

# 1. og_card.py renders a 1200x630 PNG.
"${PY[@]}" "$SK/og_card.py" --title "A reasonably long test title that should wrap" \
  --subtitle "subtitle here" --brand AllSource -o "$TMP/card.png" >/dev/null
[ "$(dims "$TMP/card.png")" = "1200x630" ] || fail "og_card not 1200x630"

# 2. optimize_image og preset -> 1200x630 WebP.
"${PY[@]}" "$SK/optimize_image.py" "$TMP/card.png" --preset og -o "$TMP/og.webp" >/dev/null
[ "$(dims "$TMP/og.webp")" = "1200x630" ] || fail "og preset not 1200x630"
file "$TMP/og.webp" | grep -qi "web/p" || fail "og output is not WebP"

# 3. hero preset -> 1600x840.
"${PY[@]}" "$SK/optimize_image.py" "$TMP/card.png" --preset hero -o "$TMP/hero.webp" >/dev/null
[ "$(dims "$TMP/hero.webp")" = "1600x840" ] || fail "hero preset not 1600x840"

# 4. raw preset honors --max-width (keeps aspect).
"${PY[@]}" "$SK/optimize_image.py" "$TMP/card.png" --preset raw --max-width 600 -o "$TMP/raw.webp" >/dev/null
[ "$(dims "$TMP/raw.webp" | cut -dx -f1)" = "600" ] || fail "raw --max-width not applied"

# 5. --format png passthrough.
"${PY[@]}" "$SK/optimize_image.py" "$TMP/card.png" --preset thumb --format png -o "$TMP/t.png" >/dev/null
[ "$(dims "$TMP/t.png")" = "800x420" ] || fail "thumb png not 800x420"

echo "PASS blog-image (5 checks)"
