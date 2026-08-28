#!/usr/bin/env bash
# Tests the web-video skill script. Dep-aware: needs ffmpeg/ffprobe; skips (exit 0)
# if absent so the suite stays green on machines without ffmpeg.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SK="$ROOT/skills/web-video/scripts"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

if ! command -v ffmpeg >/dev/null 2>&1 || ! command -v ffprobe >/dev/null 2>&1; then
  echo "SKIP web-video: ffmpeg/ffprobe not installed"
  exit 0
fi

fail() { echo "FAIL web-video: $1" >&2; exit 1; }
probe() { ffprobe -v error -select_streams v:0 -show_entries "stream=$1" -of csv=p=0 "$2"; }

# Synthetic source clip (odd-ish size to exercise even-dim scaling).
ffmpeg -y -loglevel error -f lavfi -i "testsrc=duration=2:size=801x601:rate=15" "$TMP/in.mp4"

bash "$SK/optimize_video.sh" "$TMP/in.mp4" --poster --gif --max-width 640 >/dev/null

# 1-3. all three outputs produced.
[ -f "$TMP/in-opt.mp4" ]   || fail "no optimized mp4"
[ -f "$TMP/in-poster.jpg" ] || fail "no poster frame"
[ -f "$TMP/in.gif" ]        || fail "no gif"

# 4. codec is H.264.
[ "$(probe codec_name "$TMP/in-opt.mp4")" = "h264" ] || fail "codec is not h264"

# 5. width honored AND even (H.264 requires even dims).
W="$(probe width "$TMP/in-opt.mp4")"
[ "$W" -le 640 ] || fail "width $W exceeds --max-width 640"
[ $((W % 2)) -eq 0 ] || fail "width $W is not even"

# 6. faststart: moov atom must appear before mdat for web streaming.
moov=$(grep -aob moov "$TMP/in-opt.mp4" | head -1 | cut -d: -f1)
mdat=$(grep -aob mdat "$TMP/in-opt.mp4" | head -1 | cut -d: -f1)
[ -n "$moov" ] && [ -n "$mdat" ] && [ "$moov" -lt "$mdat" ] || fail "moov atom not before mdat (faststart missing)"

# 7. audio stripped by default.
[ -z "$(ffprobe -v error -select_streams a -show_entries stream=index -of csv=p=0 "$TMP/in-opt.mp4")" ] \
  || fail "audio not stripped"

echo "PASS web-video (7 checks)"
