---
name: gcp-cloudrun
description: Deploy and operate containerized services on Google Cloud Run. The build → push → deploy → verify → rollback loop with real gcloud commands, plus config gotchas (env vars vs Secret Manager, concurrency vs CPU, cold starts, request timeout). Use when the task involves "deploy to cloud run", "cloud run config", "rollback deployment", "cloud build", or "terraform cloud run". Terraform lives in references/terraform.md; cold-start / OOM / timeout / logging debugging lives in references/diagnostics.md.
---

# Google Cloud Run deploy loop

The unit of a deploy is a **revision**: an immutable snapshot of image + config.
You never mutate a revision — you create a new one and shift traffic to it. That
single fact drives both the deploy flow and the rollback flow below.

Set these once so every command is copy-pasteable:

```bash
PROJECT=my-project
REGION=us-central1
REPO=app                              # Artifact Registry repo
SERVICE=api
IMAGE="$REGION-docker.pkg.dev/$PROJECT/$REPO/$SERVICE"
```

## 1. Build & push (Artifact Registry)

Cloud Run has one hard runtime contract: **the container must listen on
`0.0.0.0:$PORT`.** Cloud Run injects `PORT` (default `8080`); it is reserved, so
you cannot set it with `--set-env-vars`. Bind to it or the revision never becomes
ready ("failed to start and listen on the port defined by the PORT variable").

Multi-stage Dockerfile — small final image, honours `$PORT`:

```dockerfile
# ---- build ----
FROM node:20-slim AS build
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build && npm prune --production

# ---- runtime ----
FROM node:20-slim
WORKDIR /app
ENV NODE_ENV=production
COPY --from=build /app/node_modules ./node_modules
COPY --from=build /app/dist ./dist
# Listen on $PORT (Cloud Run sets it; default 8080). Do NOT hardcode a port.
CMD ["node", "dist/server.js"]
```

Create the registry once, then build & push. Prefer **Cloud Build** (builds in
GCP, no local Docker daemon, layer cache in the cloud):

```bash
# One-time: create the Docker repo
gcloud artifacts repositories create "$REPO" \
  --repository-format=docker --location="$REGION" --project="$PROJECT"

# Build + push in one step, tagged with the commit for traceability
gcloud builds submit --tag "$IMAGE:$(git rev-parse --short HEAD)" --project="$PROJECT"
```

Local Docker alternative (authenticate the daemon to Artifact Registry first):

```bash
gcloud auth configure-docker "$REGION-docker.pkg.dev"
docker build -t "$IMAGE:$(git rev-parse --short HEAD)" .
docker push "$IMAGE:$(git rev-parse --short HEAD)"
```

**Cloud Build trigger** (CI on push). `cloudbuild.yaml` uses substitution
variables — built-ins like `$PROJECT_ID`, `$SHORT_SHA`, `$COMMIT_SHA`, and custom
ones that MUST start with `_`:

```yaml
# cloudbuild.yaml
substitutions:
  _REGION: us-central1
  _SERVICE: api
steps:
  - name: gcr.io/cloud-builders/docker
    args: ['build', '-t', '${_REGION}-docker.pkg.dev/$PROJECT_ID/app/${_SERVICE}:$SHORT_SHA', '.']
  - name: gcr.io/cloud-builders/docker
    args: ['push', '${_REGION}-docker.pkg.dev/$PROJECT_ID/app/${_SERVICE}:$SHORT_SHA']
  - name: gcr.io/google.com/cloudsdktool/cloud-sdk
    entrypoint: gcloud
    args: ['run', 'deploy', '${_SERVICE}', '--image',
           '${_REGION}-docker.pkg.dev/$PROJECT_ID/app/${_SERVICE}:$SHORT_SHA',
           '--region', '${_REGION}']
images: ['${_REGION}-docker.pkg.dev/$PROJECT_ID/app/${_SERVICE}:$SHORT_SHA']
options:
  logging: CLOUD_LOGGING_ONLY
```

```bash
gcloud builds triggers create github \
  --name="$SERVICE-main" --repo-name=my-repo --repo-owner=my-org \
  --branch-pattern='^main$' --build-config=cloudbuild.yaml --project="$PROJECT"
```

## 2. Deploy

```bash
gcloud run deploy "$SERVICE" \
  --image="$IMAGE:$(git rev-parse --short HEAD)" \
  --region="$REGION" --project="$PROJECT" \
  --service-account="$SERVICE@$PROJECT.iam.gserviceaccount.com" \
  --cpu=1 --memory=512Mi --concurrency=80 \
  --min-instances=0 --max-instances=10 \
  --timeout=300 --port=8080
```

Auth — pick exactly one model:

- **Public:** `--allow-unauthenticated` (grants `roles/run.invoker` to `allUsers`).
- **IAM-gated:** `--no-allow-unauthenticated`, then grant specific callers:
  ```bash
  gcloud run services add-iam-policy-binding "$SERVICE" --region="$REGION" \
    --member="serviceAccount:caller@$PROJECT.iam.gserviceaccount.com" \
    --role="roles/run.invoker"
  ```

Config — plain config vs secrets:

- `--set-env-vars=KEY=val,KEY2=val2` for non-sensitive config. `--set-env-vars`
  **replaces** the whole set; `--update-env-vars` merges; `--remove-env-vars` drops.
- `--set-secrets=API_KEY=my-secret:latest` references **Secret Manager** — the
  value is never baked into the revision spec or image. Mount as a file with
  `--set-secrets=/secrets/key=my-secret:latest`. The runtime service account needs
  `roles/secretmanager.secretAccessor` on the secret. Never put a credential in
  `--set-env-vars`.

Sizing — see the gotchas below for the reasoning; the knobs are `--cpu`,
`--memory`, `--concurrency`, `--min-instances`, `--max-instances`, `--timeout`,
`--cpu-boost` (faster cold starts), `--no-cpu-throttling` (CPU always allocated,
for background work after the response).

Startup / liveness probes are set via the **service YAML**, not a deploy flag —
export, add the probe, and `replace`:

```yaml
# service.yaml (excerpt)
spec:
  template:
    spec:
      containers:
        - image: REGION-docker.pkg.dev/PROJECT/app/api:SHA
          startupProbe:      # gates readiness; failures block the new revision
            httpGet: { path: /healthz }
            periodSeconds: 3
            failureThreshold: 3
          livenessProbe:     # restarts a wedged container mid-serving
            httpGet: { path: /healthz }
```

```bash
gcloud run services replace service.yaml --region="$REGION"
```

**Canary / blue-green** — deploy with no traffic and a tag, verify the tagged URL,
then split traffic:

```bash
gcloud run deploy "$SERVICE" --image="$IMAGE:$SHA" --region="$REGION" \
  --no-traffic --tag=canary                       # reachable at canary---<svc>.run.app
gcloud run services update-traffic "$SERVICE" --region="$REGION" \
  --to-tags=canary=10                             # 10% to canary
gcloud run services update-traffic "$SERVICE" --region="$REGION" --to-latest
```

## 3. Verify — read the traffic state, don't trust the exit code

`gcloud run deploy` exiting `0` means the **API accepted** the request; it does not
prove the new revision is healthy AND serving traffic. Read the artifact — the
served revision — before declaring the deploy done (see
[../../rules/evidence-discipline.md](../../rules/evidence-discipline.md)):

```bash
# Which revision(s) actually receive traffic, and at what percent?
gcloud run services describe "$SERVICE" --region="$REGION" \
  --format='value(status.traffic)'

# Latest revision's readiness — must be True
gcloud run revisions list --service="$SERVICE" --region="$REGION" \
  --format='table(metadata.name, status.conditions[0].status, spec.containerConcurrency)' \
  --limit=5

# Smoke the live URL
URL=$(gcloud run services describe "$SERVICE" --region="$REGION" --format='value(status.url)')
curl -fsS "$URL/healthz"
```

A deploy is done when the **new** revision name appears in `status.traffic` at the
percent you intended AND its Ready condition is `True`. A revision stuck
`Ready=False` while an OLD revision still serves 100% is the common silent failure:
the deploy "succeeded" and none of your code is live.

## 4. Rollback — shift traffic, never mutate

Revisions are immutable, so rollback is a traffic move to a known-good revision. No
rebuild, no redeploy:

```bash
# Find the last-known-good revision
gcloud run revisions list --service="$SERVICE" --region="$REGION" --format='table(metadata.name, metadata.creationTimestamp)'

# Send 100% back to it
gcloud run services update-traffic "$SERVICE" --region="$REGION" \
  --to-revisions=api-00042-abc=100

# Re-verify with the step-3 describe — confirm the good revision now owns 100%
```

Pin traffic to explicit revisions (not `--to-latest`) in production so a stray
`gcloud run deploy` cannot silently take 100% before you have verified it.

## Config gotchas

- **PORT is a contract, not a setting.** Listen on `$PORT`; never hardcode `3000`.
  You may change the port Cloud Run sends with `--port`, but the container must
  read the env var either way.
- **Env var vs Secret Manager.** Config → `--set-env-vars`. Anything secret →
  `--set-secrets` (Secret Manager), so it stays out of the revision spec, out of
  `describe` output, and out of the image. `--set-env-vars` replaces the whole map;
  reach for `--update-env-vars` when you mean "add one".
- **Concurrency vs CPU.** `--concurrency` is requests-per-instance (default 80, max
  1000). A request-bound / I/O-bound service (mostly waiting on a DB or upstream)
  wants **high** concurrency + modest CPU — one instance serves many requests
  cheaply. A CPU-bound service (rendering, crypto, ML) wants **low** concurrency
  (even 1) + more `--cpu`, so requests don't starve each other. Setting both high
  is how you get tail-latency blowups.
- **Cold starts cost money to kill.** `--min-instances=1+` keeps warm instances but
  you pay for idle. `--cpu-boost` speeds the cold start itself without a standing
  cost. See diagnostics for the full trade.
- **Request timeout has a ceiling.** `--timeout` default is 300s, max **3600s** (60
  min). A long request that exceeds it returns **504** — distinct from a client
  timeout. Don't raise it to paper over a slow dependency.

## Time budget

Every step here must finish inside a bounded wait — a deploy that "hangs" is
usually a revision failing its startup probe, not a slow API. Do not sit on an
unbounded `gcloud run deploy`; if it does not settle, poll `revisions list` for the
`Ready` condition and read the logs (see diagnostics). Keep any scripted wait
bounded and polled, per [../../rules/timeouts.md](../../rules/timeouts.md).

## References

- [references/terraform.md](references/terraform.md) — `google_cloud_run_v2_service`,
  IAM invoker binding, service-account wiring, Serverless VPC connector,
  multi-environment state.
- [references/diagnostics.md](references/diagnostics.md) — cold starts, OOM,
  request-timeout debugging, log-based alerting.
