#!/usr/bin/env bash
# Optimize a screen recording (mp4/mov/webm) into a web-ready demo video.
#
# H.264 + faststart (moov atom up front for instant streaming), audio stripped by
# default, CRF 28 (good size/quality balance). Optional poster frame + looping GIF.
# This is the proven `ffmpeg -c:v libx264 -crf 28 -preset slow -movflags +faststart -an`
# recipe wrapped with sensible flags.
#
# Usage:
#   optimize_video.sh <input> [--crf N] [-o out.mp4] [--keep-audio] [--poster] [--gif]
#                              [--max-width N]
# Examples:
#   optimize_video.sh demo.mov
#   optimize_video.sh demo.mp4 --crf 26 --poster --gif
#   optimize_video.sh demo.webm -o public/demo.mp4 --max-width 1280
set -euo pipefail

if ! command -v ffmpeg >/dev/null 2>&1; then
  echo "ffmpeg is required. Install it with:  brew install ffmpeg   (macOS)" >&2
  echo "                                       sudo apt-get install ffmpeg   (Debian/Ubuntu)" >&2
  exit 1
fi

usage() {
  sed -n '2,16p' "$0" | sed 's/^# \{0,1\}//'
  exit "${1:-1}"
}

[ "$#" -ge 1 ] || usage
case "$1" in -h|--help) usage 0 ;; esac

IN="$1"; shift
CRF=28
AUDIO_ARGS=(-an)
OUT=""
POSTER=0
GIF=0
MAXW=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --crf)        CRF="$2"; shift 2 ;;
    -o|--out)     OUT="$2"; shift 2 ;;
    --keep-audio) AUDIO_ARGS=(); shift ;;
    --poster)     POSTER=1; shift ;;
    --gif)        GIF=1; shift ;;
    --max-width)  MAXW="$2"; shift 2 ;;
    *) echo "unknown arg: $1" >&2; usage ;;
  esac
done

[ -f "$IN" ] || { echo "no such file: $IN" >&2; exit 1; }

base="${IN%.*}"
OUT="${OUT:-${base}-opt.mp4}"

# Even dimensions are required by libx264; scale-down (keep aspect) if requested.
if [ -n "$MAXW" ]; then
  VF="scale='min($MAXW,iw)':-2"
else
  VF="scale=trunc(iw/2)*2:trunc(ih/2)*2"
fi

ffmpeg -y -i "$IN" -vf "$VF" -c:v libx264 -crf "$CRF" -preset slow -movflags +faststart -pix_fmt yuv420p "${AUDIO_ARGS[@]}" "$OUT"
printf 'wrote %s  (%s)\n' "$OUT" "$(du -h "$OUT" | cut -f1)"

if [ "$POSTER" = 1 ]; then
  ffmpeg -y -i "$OUT" -vf "select=eq(n\,0)" -frames:v 1 -q:v 3 "${base}-poster.jpg"
  printf 'wrote %s  (poster frame)\n' "${base}-poster.jpg"
fi

if [ "$GIF" = 1 ]; then
  GIF_TMPDIR="$(mktemp -d "${TMPDIR:-/tmp}/web-video-palette.XXXXXX")"
  cleanup_gif_temp() { rm -rf -- "$GIF_TMPDIR"; }
  trap cleanup_gif_temp EXIT
  pal="$GIF_TMPDIR/palette.png"
  ffmpeg -y -i "$OUT" -vf "fps=12,scale=720:-1:flags=lanczos,palettegen" "$pal"
  ffmpeg -y -i "$OUT" -i "$pal" -lavfi "fps=12,scale=720:-1:flags=lanczos[x];[x][1:v]paletteuse" "${base}.gif"
  cleanup_gif_temp
  trap - EXIT
  printf 'wrote %s  (%s)\n' "${base}.gif" "$(du -h "${base}.gif" | cut -f1)"
fi
