# Real-time media pipelines

Camera, video, and streaming pipelines have a **hard per-frame deadline**. Miss it
and the user sees dropped frames, latency, or a stalled stream — there is no
"a bit slower", only "kept up" or "fell behind". The whole review is: does each
stage finish inside its slice of the frame budget, and is anything heavy sitting
on the capture/encode path that should be elsewhere?

**Template** — fill in your pipeline's real frame rate, resolution, and stage
names. The budgets below are the physics; your allocation of them across stages
is the thing to write down and defend.

## Budgets

| Frame rate | Total per-frame budget | Meaning |
|---|---|---|
| 60 fps | **16.6 ms** | capture + process + encode + send must all fit in one frame |
| 30 fps | **33.3 ms** | the common camera-capture target |

Divide the budget across stages and measure each with `performance.now()` spans.
A stage over its slice is a finding, ranked by how much of the budget it eats.

## Checklist

| # | Pattern | Detect | Why it costs | Fix |
|---|---|---|---|---|
| M1 | Frame capture over budget | time the capture→`ImageBitmap`/`getImageData` span; is it < 16 ms (60) / 33 ms (30)? | `getImageData` and canvas readback are synchronous main-thread reads | Use `createImageBitmap` (async, off-thread decode); capture at the resolution you actually need, not the sensor max; `OffscreenCanvas` in a worker |
| M2 | Model inference on the encoder/capture path | `rg -n 'predict\(|infer|session\.run|detect\('` in the capture or encode callback | Inference is tens of ms; on the capture path it directly blows the frame budget and stalls capture | Run inference in a Worker (WASM/WebGPU/WebGL backend) on a **copy** of the frame; let capture/encode run at full rate and consume inference results when ready — never gate the next frame on inference finishing |
| M3 | WebSocket send awaiting ack before next frame | `rg -n 'await\s+.*send\(|await.*socket'` in the frame loop | Waiting for an application ack per frame serializes on the round-trip time → frame rate collapses to 1/RTT | Fire-and-forget the send; apply backpressure via `socket.bufferedAmount` (skip/drop or downshift quality when it grows), not by awaiting an ack. Decouple capture rate from network confirmation |
| M4 | Encoding on the main thread | `rg -n 'toBlob|toDataURL|VideoEncoder|encode\('` on the main thread | JPEG/H.264/WebP encoding is heavy CPU; on the main thread it competes with capture and UI | Encode in a Worker with `OffscreenCanvas` / `VideoEncoder` (WebCodecs); transfer the `ArrayBuffer` (zero-copy `postMessage` transfer list), don't clone it |
| M5 | Per-frame WASM/crypto calls not batched | `rg -n '\.encrypt\(|\.sign\(|wasm|_malloc|ccall'` in the frame loop | The JS↔WASM boundary crossing and per-call `malloc`/copy have fixed overhead paid every frame | Batch: accumulate a few frames and cross the boundary once; reuse a single WASM heap buffer (`HEAPU8.set` into a preallocated pointer) instead of allocating per call; keep the crypto context alive across frames |
| M6 | Buffer churn per frame | new typed array / canvas per frame | GC sawtooth mid-stream = periodic dropped frames | Reuse frame buffers; see [memory-allocation.md](memory-allocation.md) A5 |

## Confirm, don't guess

- Instrument each stage with a named `performance.now()` span and log the p95 per
  stage over a real session, not one frame. The budget is a distribution, not an
  average — one stage occasionally spiking over 33 ms is a visible hitch.
- Watch dropped-frame stats where the platform exposes them
  (`video.getVideoPlaybackQuality()`, WebRTC `getStats()` `framesDropped`). Falling
  behind shows there before it shows in a flame graph.
- Verify a worker offload actually moved the cost: the main-thread track should go
  quiet, not just the total shrink — instrument before concluding
  ([../../../rules/debugging-discipline.md](../../../rules/debugging-discipline.md)).
