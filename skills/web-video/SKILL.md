---
name: web-video
description: Optimize a screen recording (mp4/mov/webm) into a web-ready demo video — H.264, faststart, audio stripped — with optional poster frame and looping GIF. Use when the user has a demo/screen recording to prepare for a website, blog, or social post.
---

Turn a raw screen recording into a small, fast-loading web demo. Wraps the proven
`ffmpeg -c:v libx264 -crf 28 -preset slow -movflags +faststart -an` recipe with
sensible flags, plus optional poster frame and GIF.

## Requirements

`ffmpeg` must be installed (no pure-Python fallback exists for video encoding):

- macOS: `brew install ffmpeg`
- Debian/Ubuntu: `sudo apt-get install ffmpeg`

The script checks for `ffmpeg` and prints the install hint if it's missing.

## Running it

```
scripts/optimize_video.sh <input> [--crf N] [-o out.mp4] [--keep-audio] [--poster] [--gif] [--max-width N]
```

- **Output** defaults to `<input>-opt.mp4`: H.264, `+faststart` (streams instantly),
  `yuv420p` (plays everywhere), audio removed.
- `--crf` quality, lower = better/larger (default **28**; try 24–26 for crisper UI text).
- `--max-width N` scales down (keeps aspect, enforces even dimensions for H.264).
- `--poster` writes `<input>-poster.jpg` (first frame) — use as the `<video poster>`.
- `--gif` writes `<input>.gif` (12fps, 720px wide, palette-optimized) for places that can't embed video.
- `--keep-audio` keeps the audio track (off by default — demos rarely need it).

Examples:
```
optimize_video.sh demo.mov
optimize_video.sh demo.mp4 --crf 26 --poster --gif
optimize_video.sh demo.webm -o public/demo.mp4 --max-width 1280
```

## Workflow guidance (for the agent)

1. Always produce the optimized `.mp4` first; add `--poster` when the page uses a
   `<video>` element (set `poster="<input>-poster.jpg"`).
2. Only add `--gif` when asked or when the target can't embed video — GIFs are far
   larger than the mp4 for the same content.
3. Prefer `--max-width 1280` (or 1280→1920) for full-screen captures; UI demos read
   fine at 1280 and stay small.
4. Report each output's path and size; suggest the `<video>` snippet:
   `<video src="demo-opt.mp4" poster="demo-poster.jpg" autoplay muted loop playsinline></video>`.

## Notes

- `+faststart` moves the moov atom to the front so the video starts before it's
  fully downloaded — important for web.
- Audio is stripped by default to shave bytes; pass `--keep-audio` to keep it.
- Tested by `tests/test_web_video.sh` (H.264 codec, even/capped width, moov-before-mdat
  faststart, audio stripped, poster + gif produced).
