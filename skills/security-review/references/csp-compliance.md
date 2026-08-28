# CSP Compliance

Content-Security-Policy is a response header (or `<meta http-equiv>`) that tells
the browser which sources of script, style, and other resources are allowed. A
browser SDK embedded on a third-party page inherits **that page's** CSP — you do
not get to relax it. Code that needs `unsafe-eval` or `unsafe-inline` simply
fails to load on any hardened host. Write to the strict policy from day one.

**Read the actual policy, do not assume it.** The response header is the source
of truth; a `<meta>` tag can add a second, stricter layer. Confirm with the
Network tab or `curl -sI <url> | rg -i content-security-policy` before claiming a
directive is present. See [../../../rules/evidence-discipline.md](../../../rules/evidence-discipline.md).

A representative strict policy the SDK must survive under:

```
Content-Security-Policy:
  default-src 'self';
  script-src 'self';        (no 'unsafe-eval', no 'unsafe-inline')
  style-src 'self';         (no 'unsafe-inline')
  connect-src 'self' https://api.example.com wss://rt.example.com;
  frame-ancestors 'none';
  object-src 'none';
  base-uri 'self'
```

---

## 1. `eval` / `new Function` → parser or registry (`script-src`)

`eval`, `new Function`, the `Function` constructor, and the **string** forms of
`setTimeout`/`setInterval` all compile a string into executable code and require
`script-src 'unsafe-eval'`. Replace the dynamic behaviour with data + static
dispatch.

**Before — dynamic dispatch by generating code:**

```js
// ✗ blocked without 'unsafe-eval'; also an RCE sink if `op` is user data
function apply(op, a, b) {
  return new Function("a", "b", `return a ${op} b`)(a, b);
}
setTimeout("refresh()", 1000); // ✗ string form is eval in disguise
```

**After — a registry keyed on the input:**

```js
// ✓ CSP-safe: the operations are data, dispatch is a lookup
const OPS = {
  "+": (a, b) => a + b,
  "-": (a, b) => a - b,
  "*": (a, b) => a * b,
};
function apply(op, a, b) {
  const fn = OPS[op];
  if (!fn) throw new Error(`unsupported op: ${op}`);
  return fn(a, b);
}
setTimeout(refresh, 1000); // ✓ function reference, not a string
```

For "run this config-driven expression" needs, parse the expression into an AST /
data structure and interpret the data — never compile it to a function. JSON
config + a switch over a fixed vocabulary covers almost every real case.

---

## 2. Inline `style=` attribute → CSS class or DOM property (`style-src`)

The inline `style` **attribute** in markup requires `style-src 'unsafe-inline'`.
Two CSP-safe replacements:

**Before:**

```js
el.setAttribute("style", "color:red; display:none");   // ✗ inline style attr
element.innerHTML = `<div style="color:red">!</div>`;   // ✗ inline style attr
```

**After — option A, a stylesheet class (preferred):**

```css
/* shipped in a bundled stylesheet under script-src 'self' */
.sdk-error { color: red; }
.sdk-hidden { display: none; }
```

```js
el.classList.add("sdk-error", "sdk-hidden"); // ✓ no inline style
```

**After — option B, individual DOM style properties:**

```js
// ✓ setting the CSSStyleDeclaration property is NOT blocked by style-src;
//   only the inline `style` *attribute* in HTML is.
el.style.color = "red";
el.style.display = "none";
```

If the host page issues a per-response **nonce**, an injected `<style nonce="…">`
is allowed — but only use a nonce the page actually granted; never invent one.

---

## 3. `new Function` factory → static handler map

A common "extensible" pattern builds handlers from strings. Replace with a
registration API that stores real function references.

**Before:**

```js
const handlers = {};
function register(name, bodySource) {
  handlers[name] = new Function("event", bodySource); // ✗ eval-family
}
```

**After:**

```js
const handlers = new Map();
function register(name, fn) {
  if (typeof fn !== "function") throw new TypeError("handler must be a function");
  handlers.set(name, fn); // ✓ callers pass a function, nothing is compiled
}
function dispatch(name, event) {
  handlers.get(name)?.(event);
}
```

---

## 4. `javascript:` URIs and inline `on*=` handlers

`javascript:` in `href`/`src` and inline event attributes (`onclick="…"`) both
require `script-src 'unsafe-inline'` and are prime XSS sinks.

**Before:**

```html
<a href="javascript:submit()">go</a>                <!-- ✗ -->
<button onclick="doThing()">x</button>              <!-- ✗ inline handler -->
```

**After:**

```html
<a href="#" data-action="submit">go</a>
<button id="thing-btn">x</button>
```

```js
// ✓ addEventListener from a bundled script under script-src 'self'
document.querySelector('[data-action="submit"]')
  .addEventListener("click", (e) => { e.preventDefault(); submit(); });
document.getElementById("thing-btn")
  .addEventListener("click", doThing);
```

---

## 5. Directives worth setting on pages you DO control

If the SDK ships its own demo/host page, set these too:

- `object-src 'none'` — kills legacy plugin vectors.
- `base-uri 'self'` — stops `<base>` tag hijacking of relative URLs.
- `frame-ancestors 'none'` (or an allowlist) — anti-clickjacking, replaces the
  older `X-Frame-Options`. See [common-vulnerabilities.md](common-vulnerabilities.md).
- `connect-src` — pin the exact API and `wss://` endpoints the SDK talks to.

## Verifying CSP-safety of a build

- Load the SDK on a page whose CSP omits `unsafe-eval` and `unsafe-inline`, open
  the console, and watch for `Refused to … because it violates the following
  Content Security Policy directive` — each one names the file and directive.
- `report-uri` / `report-to` collects violations in the field; wire it in staging.
- Grep the built bundle (not just source) for `eval(`, `Function(`, and `.style`
  attribute writes — a bundler or a transitive dep can reintroduce them.
