# Common SPA Vulnerabilities

OWASP-style top issues for single-page apps and browser SDKs, with the grep lead,
the failure it causes, and the fix. These expand the checklist items — read them
when a hit needs classifying.

---

## 1. Cross-Site Scripting (XSS)

The dominant SPA bug. Untrusted data reaching an HTML-parsing sink executes as
script with the page's full privileges: session theft, request forgery, keylogging.

- **Sinks:** `innerHTML`, `outerHTML`, `insertAdjacentHTML`, `document.write`,
  `dangerouslySetInnerHTML` (React), `v-html` (Vue), jQuery `.html()`,
  `Element.setHTML` without a sanitizer config, `javascript:` URLs, and
  `<a href={userValue}>` where the value is unvalidated.
- **Grep:** `rg -n 'innerHTML|outerHTML|insertAdjacentHTML|dangerouslySetInnerHTML|v-html|document\.write|\.html\('`
- **Fix:**
  - Text → `element.textContent` / `node.append(string)`; frameworks escape
    `{value}` / `{{ value }}` by default — keep the data in that path.
  - Must render HTML → sanitize with **DOMPurify** (`DOMPurify.sanitize(dirty)`)
    immediately before assignment; do not sanitize-then-mutate.
  - Build DOM with `createElement` + `textContent` + `setAttribute` for known-safe
    attributes; never concatenate HTML strings from data.
  - Validate URL protocols before assigning to `href`/`src` — allow only
    `https:`/`mailto:`, reject `javascript:`/`data:`.
  - Adopt Trusted Types on pages you control (`require-trusted-types-for 'script'`)
    to make the dangerous sinks throw unless fed a typed, sanitized value.

---

## 2. Broken Access Control on the API the SPA calls

The single most impactful class server-side. **A hidden button is not access
control.** The SPA is fully attacker-controlled — every authorization decision
must be re-checked on the server for every request.

- **Symptoms:** the client hides an admin action but the endpoint has no role
  check; an object id in the URL (`/api/orders/1042`) returns another user's data
  (IDOR — Insecure Direct Object Reference); a JWT's claims are trusted without
  verifying the signature.
- **Fix:**
  - Enforce authz on the server for **every** endpoint; deny by default.
  - Scope every query to the authenticated principal
    (`WHERE user_id = :currentUser`), so an id from the request cannot reach
    another tenant's row.
  - Verify JWT signature and `exp`/`aud`/`iss` server-side; never trust claims
    decoded in the browser for authorization.
  - Do not rely on the client omitting a field/route to keep a user out.

---

## 3. Sensitive data in `localStorage` / `sessionStorage`

- **Why:** Web Storage is plain-text, same-origin, and readable by **any** script
  on the page — so any XSS (or a malicious third-party script) exfiltrates a token
  or key stored there. Storage has no expiry and is not sent with requests, so it
  also can't carry the `HttpOnly`/`Secure` protections a cookie can.
- **Grep:** `rg -n 'localStorage|sessionStorage|window\.name'`
- **Fix:**
  - Session tokens → server-set `HttpOnly; Secure; SameSite` cookie (see
    [web-api-security.md](web-api-security.md)), not storage.
  - Key material → non-extractable `CryptoKey` in IndexedDB (see
    [encryption-patterns.md](encryption-patterns.md)); never raw bytes in storage.
  - If a short-lived access token must live in JS memory, keep it in a closure /
    module variable, not persisted storage, and keep its lifetime minutes not days.

---

## 4. Vulnerable and outdated dependencies

A browser bundle ships its whole transitive tree to every user; one vulnerable
transitive package is a live exploit path.

- **Check:** `bun audit` (or the registry advisory feed) on every install; review
  the lockfile in PRs.
- **Fix:** patch/upgrade promptly, pin versions, drop unused packages, prefer
  small well-maintained deps. Keep the tree lean — see
  [../../../rules/dependency-hygiene.md](../../../rules/dependency-hygiene.md). Pin
  any CDN-loaded dep with SRI (see [web-api-security.md](web-api-security.md)).

---

## 5. Clickjacking / framing

An attacker frames your SPA on their page and tricks the user into clicking
through an invisible overlay ("UI redress").

- **Grep (server/headers):** `rg -ni 'frame-ancestors|X-Frame-Options'`
- **Fix:** CSP `frame-ancestors 'none'` (or an explicit allowlist) on every
  document response — it supersedes the older `X-Frame-Options: DENY`, though
  sending both covers legacy browsers. For an SDK meant to be embedded, invert it:
  validate the embedding ancestor origin server-side rather than blanket-denying.

---

## 6. Open redirects and postMessage trust

- **Open redirect:** `location = params.get("next")` sends users to an
  attacker URL under your domain's trust — validate redirect targets against an
  allowlist or require same-origin.
  - **Grep:** `rg -n 'location\s*=|location\.(href|assign|replace)\('`
- **postMessage:** a handler that skips `event.origin` accepts messages from any
  frame; and `postMessage(data, "*")` broadcasts to any listener.
  - **Grep:** `rg -n 'addEventListener\(\s*["'\'']message|postMessage\('`
  - **Fix:** check `event.origin` against an allowlist in every `message` handler,
    and always pass a specific `targetOrigin` (never `"*"`) when sending.

---

## 7. Secrets and PII leakage in the client

- Hardcoded keys/tokens in the bundle (checklist C3) — anything in JS is public.
- Verbose errors/stack traces surfaced to the user or logged to a third party.
- PII in URLs/query strings (they land in server logs, `Referer` headers, and
  browser history).
- **Fix:** keep secrets server-side; issue short-lived scoped tokens to the
  browser; log errors through a controlled channel, not raw to the console or an
  untrusted sink; keep sensitive values in the request body over TLS, not the URL.
