# Cloud Run on Terraform (`google_cloud_run_v2_service`)

Use the **v2** resource (`google_cloud_run_v2_service`), not the legacy
`google_cloud_run_service`. v2 maps 1:1 to the Cloud Run Admin API v2 — nested
`template`/`containers` blocks instead of the old annotation soup — and is the
resource Google documents for new work.

## The service

```hcl
resource "google_cloud_run_v2_service" "api" {
  name     = "api"
  location = var.region
  ingress  = "INGRESS_TRAFFIC_ALL"   # or INGRESS_TRAFFIC_INTERNAL_ONLY / _AND_CLOUD_LOAD_BALANCING

  # Guard against `terraform destroy` wiping a live prod service.
  deletion_protection = true

  template {
    service_account = google_service_account.api.email

    scaling {
      min_instance_count = 0     # >0 to kill cold starts (you pay for idle)
      max_instance_count = 10
    }

    max_instance_request_concurrency = 80    # requests per instance
    timeout                          = "300s" # request timeout, max 3600s

    containers {
      image = "${var.region}-docker.pkg.dev/${var.project}/app/api:${var.image_tag}"

      ports {
        container_port = 8080    # must match what the app listens on ($PORT)
      }

      resources {
        limits            = { cpu = "1", memory = "512Mi" }
        cpu_idle          = true   # true = throttle CPU when idle (cheaper);
                                   # false = CPU always allocated (background work)
        startup_cpu_boost = true   # faster cold starts
      }

      # Plain config
      env {
        name  = "LOG_LEVEL"
        value = "info"
      }

      # Secret Manager reference — value never lands in state as plaintext
      env {
        name = "API_KEY"
        value_source {
          secret_key_ref {
            secret  = google_secret_manager_secret.api_key.secret_id
            version = "latest"
          }
        }
      }

      startup_probe {
        http_get { path = "/healthz" }
        period_seconds    = 3
        failure_threshold = 3
      }
      liveness_probe {
        http_get { path = "/healthz" }
      }
    }

    # Private egress via a Serverless VPC connector (see below)
    vpc_access {
      connector = google_vpc_access_connector.connector.id
      egress    = "PRIVATE_RANGES_ONLY"   # or ALL_TRAFFIC
    }
  }

  traffic {
    type    = "TRAFFIC_TARGET_ALLOCATION_TYPE_LATEST"
    percent = 100
  }
}
```

For a pinned blue/green split, replace the single `traffic` block with explicit
revision targets:

```hcl
  traffic {
    type     = "TRAFFIC_TARGET_ALLOCATION_TYPE_REVISION"
    revision = "api-00042-abc"
    percent  = 90
  }
  traffic {
    type     = "TRAFFIC_TARGET_ALLOCATION_TYPE_REVISION"
    revision = "api-00043-def"
    percent  = 10
    tag      = "canary"
  }
```

## Invoker IAM

Grant the run-invoker role with `google_cloud_run_v2_service_iam_member` (keep the
resource unauthenticated in Terraform and control access here):

```hcl
# Public
resource "google_cloud_run_v2_service_iam_member" "public" {
  name     = google_cloud_run_v2_service.api.name
  location = google_cloud_run_v2_service.api.location
  role     = "roles/run.invoker"
  member   = "allUsers"
}

# Or a specific caller (service-to-service)
resource "google_cloud_run_v2_service_iam_member" "caller" {
  name     = google_cloud_run_v2_service.api.name
  location = google_cloud_run_v2_service.api.location
  role     = "roles/run.invoker"
  member   = "serviceAccount:${google_service_account.caller.email}"
}
```

Use `_iam_member` (additive, one binding) over `_iam_policy` (authoritative,
overwrites every binding) unless you truly own the whole policy.

## Service account wiring

Run each service as its own least-privilege identity — never the default compute SA:

```hcl
resource "google_service_account" "api" {
  account_id   = "api-runtime"
  display_name = "Cloud Run runtime SA for api"
}

# Let the runtime SA read the secret referenced above
resource "google_secret_manager_secret_iam_member" "api_key_access" {
  secret_id = google_secret_manager_secret.api_key.secret_id
  role      = "roles/secretmanager.secretAccessor"
  member    = "serviceAccount:${google_service_account.api.email}"
}
```

## Serverless VPC Access connector

Needed for the service to reach private resources (Cloud SQL private IP, internal
LB, Memorystore):

```hcl
resource "google_vpc_access_connector" "connector" {
  name          = "run-connector"
  region        = var.region
  network       = "default"
  ip_cidr_range = "10.8.0.0/28"   # a /28 dedicated to the connector
  min_instances = 2
  max_instances = 3
  machine_type  = "e2-micro"
}
```

Reference it from the service via the `vpc_access` block shown above. Enable the
`vpcaccess.googleapis.com` API first.

## Multi-environment state

- **One state per environment**, isolated by backend prefix, so a `dev` apply can
  never touch `prod`:

  ```hcl
  terraform {
    backend "gcs" {
      bucket = "my-tf-state"
      prefix = "cloudrun/prod"   # cloudrun/dev, cloudrun/staging in sibling configs
    }
  }
  ```

- Prefer **separate root modules / directories per env** (`envs/dev`, `envs/prod`)
  over `terraform workspace` — a wrong `terraform workspace select` is a silent,
  easy way to apply prod changes to dev or vice-versa; a directory boundary is not.
- Factor the service into a **shared child module** and pass `project`, `region`,
  `image_tag`, and sizing as variables so environments diverge only in their
  `.tfvars`, not in copy-pasted resource blocks.
