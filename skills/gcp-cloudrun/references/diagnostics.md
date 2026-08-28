# Cloud Run diagnostics

Read the actual log line and the actual revision state before theorizing about a
cause — the error message names the failure, and a Cloud Run failure is almost
always one of the four below wearing a generic 5xx. See
[../../../rules/debugging-discipline.md](../../../rules/debugging-discipline.md).

Pull the service's logs directly (works everywhere; no beta surface needed):

```bash
gcloud logging read \
  'resource.type=cloud_run_revision AND resource.labels.service_name=api AND severity>=WARNING' \
  --project="$PROJECT" --limit=50 --freshness=1h \
  --format='table(timestamp, severity, textPayload)'
```

## Cold starts

Symptom: the first request after idle takes seconds; p99 latency spikes after quiet
periods. Cold start = pull image → start container → pass startup probe.

Fixes, cheapest signal first:

- **Slim the image.** A 1.2 GB image pulls far slower than a 120 MB one. Multi-stage
  build, distroless/`-slim` runtime base, copy only artifacts (see the SKILL).
- **`--cpu-boost`** (`startup_cpu_boost = true` in Terraform) — extra CPU during
  startup only, no standing cost. First reach for this.
- **`--min-instances=N`** keeps N instances warm — eliminates cold starts entirely
  but **you pay for idle instances 24/7**. Use for latency-critical services, not
  everything.
- **Lazy-init in code.** Don't open every DB pool / load every model at import time
  if the first request doesn't need it — move it behind first use so the startup
  probe passes fast.

## OOM — memory limit exceeded

Symptom: instances die mid-request, requests return 503, and the log carries:

```
Memory limit of 512 MiB exceeded with 530 MiB used. Consider increasing the memory limit.
```

That line is authoritative — the container was killed, it did not crash on its own.
Two real causes, opposite fixes:

- **Legitimately under-provisioned** (large uploads buffered in memory, a big
  in-memory cache): raise `--memory` (e.g. `512Mi` → `1Gi`).
- **A leak** — memory climbs monotonically across requests until the limit. Raising
  the limit only buys time. Confirm by watching the *Container memory utilization*
  metric trend over hours; a sawtooth that never returns to baseline is a leak.
  Fix the leak; don't chase it with `--memory`.

Note `/tmp` on Cloud Run is an **in-memory tmpfs** — files written there count
against the memory limit. Writing a large temp file reads as an OOM.

## Request timeout — service vs client

Two different timeouts return two different errors; identify which fired before
touching anything:

- **Service timeout** (`--timeout`, default 300s, max 3600s): Cloud Run terminates
  the request and returns **504**, logged as *"The request has been terminated
  because it has reached the maximum request timeout."* The fix is rarely a bigger
  `--timeout` — it is a slow dependency, an N+1 query, or work that belongs in a
  background task / Cloud Tasks queue instead of the request path.
- **Client timeout**: the caller (browser, load balancer, SDK) gave up first. The
  Cloud Run log shows the request still running or completing normally. Raising
  `--timeout` does nothing here — fix the client's deadline or the latency.

Read the log to see which side cut the connection; don't guess from the symptom.

Also check `--concurrency`: if one instance is pinned at its concurrency limit,
extra requests **queue** and can breach the client's deadline while each individual
request is fast. That's a sizing problem (raise concurrency or max-instances), not
a timeout problem.

## The "won't start" failure

Symptom: `gcloud run deploy` reports the revision is not ready, log says:

```
The user-provided container failed to start and listen on the port defined
provided by the PORT=8080 environment variable within the allocated timeout.
```

Nearly always one of: the app binds `127.0.0.1` instead of `0.0.0.0`; it hardcodes
a port instead of reading `$PORT`; or it takes too long to become ready — add a
`startupProbe` with a generous `failureThreshold` so slow-but-healthy boots aren't
killed.

## Log-based alerting

Turn a recurring error log into a metric, then alert on it. With gcloud:

```bash
# 1. A counter metric over matching log entries
gcloud logging metrics create run_5xx \
  --description="Cloud Run 5xx responses for api" \
  --log-filter='resource.type=cloud_run_revision AND resource.labels.service_name=api AND httpRequest.status>=500'
```

Then create an alert policy on `logging.googleapis.com/user/run_5xx` in Cloud
Monitoring. The same pair expressed in Terraform (keeps the alert in version
control alongside the service):

```hcl
resource "google_logging_metric" "run_5xx" {
  name   = "run_5xx"
  filter = "resource.type=cloud_run_revision AND resource.labels.service_name=api AND httpRequest.status>=500"
  metric_descriptor {
    metric_kind = "DELTA"
    value_type  = "INT64"
  }
}

resource "google_monitoring_alert_policy" "run_5xx" {
  display_name = "api 5xx rate"
  combiner     = "OR"
  conditions {
    display_name = "5xx over threshold"
    condition_threshold {
      filter          = "metric.type=\"logging.googleapis.com/user/${google_logging_metric.run_5xx.name}\" resource.type=\"cloud_run_revision\""
      comparison      = "COMPARISON_GT"
      threshold_value = 5
      duration        = "300s"
      aggregations {
        alignment_period   = "60s"
        per_series_aligner = "ALIGN_RATE"
      }
    }
  }
  notification_channels = [google_monitoring_notification_channel.oncall.id]
}
```

Alert on a **rate over a window** (as above), not on a single occurrence — one 500
during a deploy is noise; five per minute for five minutes is an incident.
