---
title: Network Debugging
tags: http, websocket, grpc, curl, tls, connection-refused, timeout
---

# Network Debugging

First, classify the failure — the class tells you the layer. These three are
distinct and demand different fixes:

| Symptom | Layer | Meaning |
|---|---|---|
| **Connection refused** (immediate) | transport | Nothing is listening on that host:port. Wrong port, service down, not bound. |
| **Timeout** (hangs, then fails) | transport / network | Something is listening but not answering, or a firewall/security group is dropping packets silently. |
| **TLS / cert error** | transport (secure) | Reached the peer, handshake failed. Wrong CA, expired cert, SNI/hostname mismatch. See environment-debugging. |

"Refused" is fast and local; "timeout" is slow and usually a firewall or an
overloaded peer. Do not conflate them.

## HTTP

Read the status CLASS before the body:
- **2xx** success — if the client still errored, the bug is in parsing/deserialization, not the call.
- **3xx** redirect — is the client following it? A redirected POST silently becomes a GET on many clients.
- **4xx** you sent something wrong — auth (401/403), not found (404), bad payload (400/422), rate limit (429). Fix the request.
- **5xx** the server failed — now go read the SERVER's logs; the client is a bystander.

```bash
# -v shows request+response headers, TLS handshake, redirects
curl -v https://api.example.com/health
# just the status and timing, no body
curl -s -o /dev/null -w 'status=%{http_code} dns=%{time_namelookup} connect=%{time_connect} tls=%{time_appconnect} ttfb=%{time_starttransfer}\n' https://api.example.com/health
```

The `-w` timings split the failure: high `time_namelookup` → DNS; `connect` stalls →
transport/firewall; `time_appconnect` stalls → TLS; high `ttfb` → the server is slow,
not the network.

## WebSocket

A WS connection is an HTTP upgrade first — if the handshake fails you never had a
socket.
- Handshake needs `Connection: Upgrade`, `Upgrade: websocket`, and a `101 Switching
  Protocols` response. A `200` or `404` back means the route is not a WS endpoint.
- Read the **close code**: `1000` normal; `1006` abnormal (no close frame — usually a
  dropped TCP connection or a proxy killing an idle socket); `1011` server error;
  `1008`/`4xxx` policy/app-defined. `1006` + a reverse proxy usually means an idle
  timeout — add a heartbeat/ping.

## gRPC

- Read the **status code** (not HTTP): `UNAVAILABLE` (14) = transport, retry-able,
  often the same "refused vs timeout" split as above; `DEADLINE_EXCEEDED` (4) = your
  timeout fired; `UNIMPLEMENTED` (12) = wrong method/version or reflection off;
  `UNAUTHENTICATED` (16)/`PERMISSION_DENIED` (7) = creds.
- Use **reflection** to confirm the server exposes what you call, before blaming your
  stub:

```bash
grpcurl -plaintext localhost:50051 list          # services
grpcurl -plaintext localhost:50051 describe pkg.Service.Method
```

`UNIMPLEMENTED` on `list` means reflection is disabled, not that the server is broken.

## Trap

`connect exit 0` / "request sent" is not "response received." Verify the response
body/status at the destination — a proxy or load balancer can accept and then drop.
See [../../../rules/evidence-discipline.md](../../../rules/evidence-discipline.md).
