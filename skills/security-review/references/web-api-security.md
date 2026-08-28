# Web API Security

Concrete secure-vs-insecure snippets for the browser/server boundary an SDK
crosses: WebSocket, fetch/CORS, cookies, and Subresource Integrity.

**Every header value here is a runtime fact.** Read the actual response
(`curl -sI`, the Network tab, or the server config) before concluding a flag is
set or a policy is correct — see [../../../rules/evidence-discipline.md](../../../rules/evidence-discipline.md).

---

## WebSocket

Two independent requirements: encrypted transport, and server-side origin +
auth checks. The browser gives you neither for free.

```js
// ✗ insecure: cleartext; blocked as mixed content on an HTTPS page anyway
const ws = new WebSocket("ws://rt.example.com/stream");

// ✓ TLS
const ws = new WebSocket("wss://rt.example.com/stream");
```

The critical part is **server-side**. The WebSocket handshake is not governed by
the same-origin policy: the browser sends an `Origin` header but does **not**
block cross-origin WebSocket connections. A server that skips the check accepts
sockets from any website (Cross-Site WebSocket Hijacking), riding the victim's
cookies.

```js
// ✓ server upgrade handler — validate Origin against an allowlist, then auth
const ALLOWED = new Set(["https://app.example.com", "https://sdk.example.com"]);

server.on("upgrade", (req, socket, head) => {
  const origin = req.headers.origin;
  if (!ALLOWED.has(origin)) {          // exact match, not substring/startsWith
    socket.write("HTTP/1.1 403 Forbidden\r\n\r\n");
    socket.destroy();
    return;
  }
  // authenticate the session (token in the first app-level message or a
  // short-lived ticket in the URL — NOT long-lived credentials in the URL,
  // which land in logs). Only then complete the upgrade.
});
```

Do not authenticate solely by cookie: cookies are attached automatically on the
handshake, which is exactly what the hijack abuses. Require an explicit token the
attacker's page cannot read.

---

## fetch / CORS

CORS is enforced by the **browser** but configured by the **server**. Get the
server side right — the browser cannot protect you from a permissive server.

```js
// ✗ server: reflecting the caller's Origin turns CORS off for everyone
res.setHeader("Access-Control-Allow-Origin", req.headers.origin);
res.setHeader("Access-Control-Allow-Credentials", "true");

// ✗ '*' with credentials — browsers reject this combo, so code that appears to
//    work is silently reflecting the Origin (the bug above) instead
res.setHeader("Access-Control-Allow-Origin", "*");
res.setHeader("Access-Control-Allow-Credentials", "true");
```

```js
// ✓ server: allowlist, echo only a matched origin, and Vary so caches don't
//   serve one origin's ACAO to another
const ALLOWED = new Set(["https://app.example.com"]);
const origin = req.headers.origin;
if (ALLOWED.has(origin)) {
  res.setHeader("Access-Control-Allow-Origin", origin);
  res.setHeader("Access-Control-Allow-Credentials", "true");
  res.setHeader("Vary", "Origin");
}
```

Client side:

```js
// ✓ only send credentials cross-origin when you truly need cookies/auth on
//   that request; default is "same-origin"
await fetch("https://api.example.com/me", { credentials: "include" });
```

- Keep `Access-Control-Allow-Methods` / `-Headers` minimal — list only what the
  API uses, not `*`.
- `*` for `Allow-Origin` is acceptable only for genuinely public, credential-less
  endpoints.

---

## Cookies

Session cookies must be set by the **server** with all three flags; JS cannot set
`HttpOnly`, which is the point.

```
✗  Set-Cookie: sid=abc123
✓  Set-Cookie: sid=abc123; Secure; HttpOnly; SameSite=Lax; Path=/; Max-Age=3600
```

| Flag | Protects against | Notes |
|---|---|---|
| `HttpOnly` | XSS reading the cookie via `document.cookie` | server-only; not settable from JS |
| `Secure` | leaking over plain HTTP | required for `SameSite=None` |
| `SameSite=Lax`/`Strict` | CSRF (cross-site sends) | `None` re-opens CSRF and needs its own token defense |
| `Path` / `Max-Age` | over-broad scope / lifetime | scope tightly |

```js
// ✗ a session token in client-readable storage or a non-HttpOnly cookie is
//    stealable by any XSS on the page
document.cookie = "sid=abc123";       // cannot be HttpOnly — never for sessions
localStorage.setItem("token", jwt);   // readable by any injected script
```

Prefix-lock sensitive cookies with `__Host-` (requires `Secure`, `Path=/`, no
`Domain`) so a subdomain cannot overwrite them.

---

## Subresource Integrity (SRI)

Any script/style you load from a CDN or other origin must carry an `integrity`
hash so a compromised or MITM'd CDN cannot swap in attacker code.

```html
<!-- ✗ no integrity: whatever the CDN serves runs with your page's privileges -->
<script src="https://cdn.example.com/lib@1.2.3/lib.min.js"></script>

<!-- ✓ pinned to a hash; browser refuses the file if the bytes don't match -->
<script
  src="https://cdn.example.com/lib@1.2.3/lib.min.js"
  integrity="sha384-oqVuAfXRKap7fdgcCY5uykM6+R9GqQ8K/uxy9rx7HNQlGYl1kPzQho1wx4JwY8wC"
  crossorigin="anonymous"></script>
```

- `crossorigin="anonymous"` is required for SRI on cross-origin resources —
  without it the browser cannot read the response to hash it.
- Generate the hash from the exact bytes:
  `cat lib.min.js | openssl dgst -sha384 -binary | openssl base64 -A`.
- SRI pins one version — pair it with a pinned URL, never a `@latest` URL.
- Even better: self-host and bundle the dependency so there is no third-party
  fetch to protect — then there is no external byte to pin at all.
- There is no reliable "require SRI everywhere" CSP directive today (the old
  `require-sri-for` proposal was removed from browsers), so enforce it in review:
  grep the built HTML for CDN `src=`/`href=` without `integrity=`.
