---
title: Log Analysis
tags: logs, tracing, pino, winston, correlation-id, log-levels
---

# Log Analysis

Logs are the cheapest instrument you have. The discipline is reading them in the
right ORDER and at the right LEVEL — not scrolling to the bottom and reacting to
whatever is there.

## Find the FIRST error, not the last

The last error is usually a symptom of the first. A dropped DB connection at
`10:00:01` produces a cascade of "query failed", "handler 500", "circuit open" for
the next thirty seconds — fixing the last line fixes nothing.

- Sort ascending and read the FIRST `error`/`fatal` in the failing window. That is
  the one to explain; treat everything after it as fallout until proven otherwise.
- A stack trace's root cause is at the BOTTOM (the `Caused by:` / innermost frame),
  even though the message you saw first is at the top.

## Read structured logs as data, not text

Modern logs are JSON (pino, winston) or key-value spans (Rust `tracing`). Query
them; do not eyeball them.

```bash
# pino / winston JSON — first error in the window, fields only
jq -c 'select(.level=="error" or ((.level|type=="number") and .level>=50)) | {t:.time, msg, req_id, err:.err.message}' app.log | head

# Rust tracing (JSON via tracing-subscriber) — one request's spans, in order
jq -c 'select(.span.request_id=="abc123") | {ts:.timestamp, lvl:.level, target, msg:.fields.message}' app.log
```

pino uses numeric levels (`50`=error, `60`=fatal); winston uses string levels by
default. Rust `tracing` levels are `TRACE < DEBUG < INFO < WARN < ERROR`.

## Follow the correlation id

One request touches many services and threads; without a shared id the lines are
noise. Grab the `request_id` / `trace_id` / `x-request-id` from the first error and
filter EVERYTHING to it. If your logs have no correlation id, that is the first fix —
you cannot debug distributed flow without one.

## Level is a signal, not decoration

- `ERROR`/`FATAL` — something failed. Explain every one in your window.
- `WARN` — a degraded path was taken (retry, fallback, cache miss). A burst of WARN
  right before an ERROR is often the actual cause.
- `INFO` — lifecycle milestones; use them to bound the failing window.
- `DEBUG`/`TRACE` — usually off in prod. If you need them, that is instrumentation:
  add the line, raise the level for the one target, reproduce, read.

## Traps (evidence-discipline)

- **A 0-byte log is not "nothing happened."** Output may be buffered, or not a tty.
  Confirm the process writes where you are reading before concluding it is idle.
- **A filter that matches nothing reads exactly like a clean run.** If `grep`/`jq`
  returns empty, prove the filter works against a line you KNOW exists before
  trusting the emptiness.
- **The log is testimony, not proof of behavior.** A log line saying "wrote 3 rows"
  is a claim; the 3 rows in the store are the evidence. Cross-check when it matters.
