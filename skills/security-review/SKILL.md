---
name: security-review
description: "Review web, SPA, and browser SDK code for security. Use for CSP compliance, credential exposure, XSS, SQL or command injection, WebCrypto correctness, CORS, cookie flags, SRI, WebSockets, dependency choices, vulnerability checks, security audits, and pre-release review."
---

# Security Review

A severity-tagged checklist for web front-ends, single-page apps, and browser
SDKs shipped to third-party sites. The threat model: your code runs inside a
strict Content-Security-Policy on a page you do not control, next to untrusted
scripts, and must never leak a credential or hand an attacker a script-injection
primitive.

Work top to bottom. For each item: **grep for the pattern, judge each hit, apply
the fix.** A grep hit is a lead, not a verdict — a `.innerHTML = "static string"`
is fine; `.innerHTML = userInput` is a bug. Classify every hit.

**Never assert a header or config value from memory.** CSP, CORS, and cookie
flags are runtime facts — read the actual response header or the actual config
file before reporting on it. See [../../rules/evidence-discipline.md](../../rules/evidence-discipline.md).

---

## CRITICAL — must block the merge

### C1. No `eval` / `new Function` (CSP `script-src` violation)

- **Grep:** `rg -n '\beval\s*\(|new\s+Function\s*\(|setTimeout\s*\(\s*["'\'']|setInterval\s*\(\s*["'\'']'`
- **Why:** `eval`, `new Function`, and string-argument `setTimeout`/`setInterval`
  compile a string into code. A page with `script-src` that lacks `unsafe-eval`
  (any hardened CSP) throws and your SDK dies on load. It is also the classic RCE
  primitive if the string is attacker-influenced.
- **Fix:** Replace dynamic dispatch with static dispatch — a lookup table /
  registry keyed on a string, or a parser that returns data instead of code.
  Pass a function reference to timers, never a string. See
  [references/csp-compliance.md](references/csp-compliance.md).

### C2. No inline `style=` attributes, `javascript:` URIs, or `unsafe-inline` reliance

- **Grep:** `rg -n 'style\s*=\s*["'\'']|\.setAttribute\(\s*["'\'']style|javascript:|href\s*=\s*["'\'']\s*javascript:'`
- **Why:** Inline styles require `style-src 'unsafe-inline'`; `javascript:` URIs
  and inline `on*=` handlers require `script-src 'unsafe-inline'`. Both weaken the
  policy for the whole host page and are what a strict CSP exists to forbid.
- **Fix:** Move styles to a stylesheet class, or set individual properties via
  `element.style.color = ...` (the DOM property assignment is not blocked by
  `style-src`; the inline `style` *attribute* is). Use a nonce/hash only if the
  host page grants one. Never build a `javascript:` URL. Details in
  [references/csp-compliance.md](references/csp-compliance.md).

### C3. No hardcoded secrets

- **Grep:** `rg -ni 'api[_-]?key|secret|token|password|passwd|private[_-]?key|authorization|bearer\s' --glob '!*.test.*'` then `rg -n '[A-Za-z0-9_\-]{32,}'` on suspect files.
- **Why:** Anything in a browser bundle is public — shipping a secret leaks it to
  every user and every `view-source`. Git history keeps it even after removal.
- **Fix:** Move the secret server-side; the browser gets a short-lived,
  scoped token issued by your backend. If one already shipped, treat it as
  compromised and **rotate it** — deleting the line is not enough.

### C4. No XSS injection sinks fed by untrusted data

- **Grep:** `rg -n 'innerHTML|outerHTML|insertAdjacentHTML|dangerouslySetInnerHTML|document\.write|\.html\('`
- **Why:** Assigning untrusted data to any of these parses it as HTML and runs
  embedded scripts/handlers — stored or reflected XSS. `dangerouslySetInnerHTML`
  (React) and jQuery `.html()` are the same sink.
- **Fix:** Use `textContent` for text. If you must render HTML, sanitize with a
  vetted library (DOMPurify) and assign the sanitized result. Prefer building DOM
  nodes with `createElement` + `textContent`. See
  [references/common-vulnerabilities.md](references/common-vulnerabilities.md).

### C5. No SQL / command injection (server side of the SDK)

- **Grep:** `rg -n 'query\(\s*`|exec\(|execSync|spawn\(|child_process|`.*\$\{.*\}.*`'` in server/BFF code.
- **Why:** String-concatenated SQL or a shell command built from request data is
  the oldest RCE/data-exfiltration bug there is.
- **Fix:** Parameterized queries / prepared statements only — never string
  interpolation into SQL. For shells, pass an argv array to `spawn` (no shell), or
  avoid the shell entirely. See
  [references/common-vulnerabilities.md](references/common-vulnerabilities.md).

### C6. Encryption done right — no ECB, no weak algorithms

- **Grep:** `rg -ni 'ecb|createCipher\b|MD5|SHA-?1\b|\bDES\b|RC4|Math\.random'`
- **Why:** ECB leaks plaintext structure (identical blocks → identical
  ciphertext). MD5/SHA-1 are broken for integrity/signatures. DES/RC4 are dead.
  `Math.random()` is not cryptographically secure. Node's `createCipher`
  (no `iv`) derives a key insecurely — use `createCipheriv`.
- **Fix:** AES-256-**GCM** via WebCrypto `SubtleCrypto`, a fresh random 96-bit IV
  per message from `crypto.getRandomValues`, SHA-256+ for hashing. Full patterns
  in [references/encryption-patterns.md](references/encryption-patterns.md).

---

## HIGH — fix before shipping

### H1. CORS not reflecting Origin, and never `*` with credentials

- **Grep (server):** `rg -n 'Access-Control-Allow-Origin|Access-Control-Allow-Credentials|cors\('`
- **Why:** Reflecting the request `Origin` back verbatim, or `Allow-Origin: *`
  together with `Allow-Credentials: true`, lets any site make authenticated
  cross-origin calls as the victim. (`*` + credentials is actually rejected by
  browsers, so a server that "works" here is reflecting the Origin — the real
  hole.)
- **Fix:** Check the request Origin against a strict allowlist and echo only
  matched values. See [references/web-api-security.md](references/web-api-security.md).

### H2. Cookie flags: `Secure`, `HttpOnly`, `SameSite`

- **Grep:** `rg -n 'Set-Cookie|document\.cookie|cookie\s*:'`
- **Why:** A session cookie without `HttpOnly` is readable by any XSS; without
  `Secure` it rides over plain HTTP; without `SameSite` it is sent on cross-site
  requests (CSRF). `document.cookie` in client code cannot set `HttpOnly` at all —
  session cookies must be set server-side.
- **Fix:** `Set-Cookie: id=…; Secure; HttpOnly; SameSite=Lax` (or `Strict`) from
  the server. See [references/web-api-security.md](references/web-api-security.md).

### H3. Subresource Integrity on third-party/CDN scripts

- **Grep:** `rg -n '<script[^>]*src=|<link[^>]*href=' --glob '*.html'` — flag any CDN URL without `integrity=`.
- **Why:** A CDN script with no `integrity` hash means a CDN compromise or
  MITM silently runs attacker code with your page's privileges.
- **Fix:** Add `integrity="sha384-…"` + `crossorigin="anonymous"`. Better, self-host
  and bundle. See [references/web-api-security.md](references/web-api-security.md).

### H4. WebSocket: `wss://` only, plus server-side origin validation

- **Grep:** `rg -n 'new WebSocket\(|ws://|WebSocket\.Server|\.upgrade\('`
- **Why:** `ws://` is unencrypted and blocked as mixed content on HTTPS pages.
  The WebSocket handshake is **not** subject to the same-origin policy — the
  browser sends an `Origin` header but does not enforce it, so a server that
  skips the origin check accepts connections from any site (cross-site hijacking).
- **Fix:** Always `wss://`. Validate the `Origin` header against an allowlist in
  the server's upgrade handler, and authenticate the session. See
  [references/web-api-security.md](references/web-api-security.md).

---

## MEDIUM — prefer the safer choice

### M1. Prefer `@bufbuild/protobuf` (protobuf-es) over `google-protobuf`

- **Grep:** `rg -n 'google-protobuf|require\(.google-protobuf|from .google-protobuf' package.json` (and lockfile).
- **Why:** The `google-protobuf` JS runtime uses `new Function()` in its
  reflection/serialization paths, which a strict CSP without `unsafe-eval`
  blocks — the SDK breaks on hardened host pages. `protobuf-es` is generated,
  tree-shakeable, and CSP-safe (no `eval`/`Function`).
- **Fix:** Migrate to `@bufbuild/protobuf` + `@bufbuild/protoc-gen-es`. Keeping the
  dep list lean also helps [../../rules/dependency-hygiene.md](../../rules/dependency-hygiene.md).

### M2. Prefer platform WebCrypto (or WASM) over hand-rolled JS crypto

- **Grep:** `rg -ni 'crypto-js|jsencrypt|forge\b|node-forge'`
- **Why:** WebCrypto (`crypto.subtle`) runs in native code, is reviewed, and its
  primitives are far less exposed to timing side-channels than pure-JS crypto,
  where the JIT and GC make constant-time impossible to guarantee. When WebCrypto
  lacks a primitive you need, a WASM implementation is the next best thing — a
  fixed compilation target is closer to constant-time than JIT'd JS.
- **Fix:** Use `crypto.subtle` for AES-GCM, ECDH, HMAC, digests. Reach for a WASM
  build only for what WebCrypto does not cover. See
  [references/encryption-patterns.md](references/encryption-patterns.md).

---

## Output shape (pass / fail)

Report findings grouped by severity, most severe first. **Empty output means the
review passed** — do not pad it. For each finding give: severity, one-line claim,
`file:line`, and the concrete fix.

```
SECURITY REVIEW — <target>

CRITICAL
  [C4] XSS: untrusted `msg.body` assigned to innerHTML
       src/ui/toast.ts:88
       → sanitize with DOMPurify, or use textContent

HIGH
  [H2] Session cookie set without HttpOnly/SameSite
       server/auth.ts:41
       → Set-Cookie: …; Secure; HttpOnly; SameSite=Lax

MEDIUM
  (none)
```

Clean run:

```
SECURITY REVIEW — <target>
No findings. CRITICAL/HIGH/MEDIUM all clear.
```

## Composability

This is the security lane of a review. When a general code-review skill runs, it
delegates security judgement here and merges these findings into its report under
the same severity buckets; it does not re-derive them. Keep the finding format
above stable so the two compose without translation.

## References

- [references/csp-compliance.md](references/csp-compliance.md) — CSP rules with before/after refactors.
- [references/encryption-patterns.md](references/encryption-patterns.md) — correct WebCrypto AES-GCM, IV rules, why ECB fails.
- [references/web-api-security.md](references/web-api-security.md) — WebSocket, CORS/fetch, cookies, SRI, secure vs insecure snippets.
- [references/common-vulnerabilities.md](references/common-vulnerabilities.md) — OWASP-style SPA top issues.
