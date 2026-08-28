# Encryption Patterns (WebCrypto)

Use the platform `crypto.subtle` (WebCrypto) API. It runs in native code, is
audited, and keeps key material out of reach of JS. Do not hand-roll crypto or
pull in a pure-JS cipher library when WebCrypto covers the primitive — see M2 in
the checklist.

WebCrypto is available at `window.crypto.subtle` (browsers, secure contexts only —
HTTPS or localhost) and `globalThis.crypto.subtle` (Node 15+, Bun). `SubtleCrypto`
methods are all async and return promises.

---

## AES-256-GCM: the default symmetric cipher

GCM is authenticated encryption — it gives confidentiality **and** integrity in
one pass, so a tampered ciphertext fails to decrypt instead of returning garbage.

```js
// Generate a 256-bit AES-GCM key
const key = await crypto.subtle.generateKey(
  { name: "AES-GCM", length: 256 },
  false,                 // extractable: false — key cannot be read back out
  ["encrypt", "decrypt"]
);

// Encrypt. A FRESH random 96-bit (12-byte) IV per message. Never reuse one.
async function encrypt(key, plaintext /* Uint8Array */) {
  const iv = crypto.getRandomValues(new Uint8Array(12)); // 96-bit nonce
  const ciphertext = await crypto.subtle.encrypt(
    { name: "AES-GCM", iv },        // tagLength defaults to 128 bits — keep it
    key,
    plaintext
  );
  return { iv, ciphertext };        // ship the IV alongside; it is not secret
}

async function decrypt(key, iv, ciphertext) {
  // Throws OperationError if the auth tag fails — i.e. the data was tampered.
  return crypto.subtle.decrypt({ name: "AES-GCM", iv }, key, ciphertext);
}
```

### The IV rule (this is the one that gets people)

**Never reuse an (IV, key) pair.** GCM catastrophically fails on nonce reuse: two
messages under the same key and IV let an attacker XOR out the keystream AND
forge the authentication tag for that key. So:

- 96 bits is the GCM-recommended IV size — use exactly 12 random bytes.
- Generate the IV with `crypto.getRandomValues` for **every** message.
- The IV is not secret — prepend/store it with the ciphertext. What must never
  repeat is the *pair*, not the IV's secrecy.
- A fixed or counter-from-zero IV baked into the source is a critical bug.
- At very high message volumes prefer a per-key random IV plus key rotation, or
  a dedicated nonce-misuse-resistant mode; random 96-bit IVs are safe up to ~2^32
  messages per key.

---

## Randomness: `crypto.getRandomValues`, never `Math.random`

```js
// ✓ cryptographically secure
const salt = crypto.getRandomValues(new Uint8Array(16));
const token = crypto.randomUUID();          // secure random v4 UUID

// ✗ NOT secure — predictable PRNG, never for keys/IVs/tokens/salts
const bad = Math.random().toString(36);
```

`Math.random` is a fast non-cryptographic PRNG; its output is predictable and must
never seed a key, IV, salt, session token, or password-reset code.

---

## Why ECB is broken

ECB (Electronic Codebook) encrypts each 16-byte block independently with no IV, so
**identical plaintext blocks produce identical ciphertext blocks.** Structure
leaks straight through — the canonical demo is the "ECB penguin", where an
encrypted bitmap is still visibly the penguin. ECB also provides no integrity.
Never select it; never use a cipher API that has no IV parameter.

WebCrypto deliberately does **not** offer ECB — if you see ECB it came from a
JS library or a misused Node `crypto`. Node's legacy `crypto.createCipher(algo,
password)` (no explicit IV, key derived via MD5) is likewise unsafe; use
`crypto.createCipheriv(algo, key, iv)` with a real key and random IV.

---

## Hashing and integrity: SHA-256+, never MD5/SHA-1

```js
const digest = await crypto.subtle.digest(
  "SHA-256",                                  // or SHA-384 / SHA-512
  new TextEncoder().encode(message)
);
```

- MD5 and SHA-1 are broken for collision resistance — never use them for
  signatures, integrity, or deduplication of untrusted data. WebCrypto's
  `digest` does not even offer MD5, and SHA-1 exists only for legacy interop.
- For message authentication use **HMAC-SHA-256** (`crypto.subtle.sign`/`verify`
  with `{ name: "HMAC", hash: "SHA-256" }`), not a bare hash of `key || message`.
- For password storage (server side) use a slow KDF — Argon2id, scrypt, or
  bcrypt — never a plain fast hash. WebCrypto `deriveBits` with PBKDF2 is the
  browser-available option when you must derive a key from a passphrase; use a
  high iteration count and a random salt.

---

## Key handling

- **Prefer non-extractable keys** (`extractable: false`) so JS/XSS cannot read the
  raw bytes; the key stays a handle you can only `encrypt`/`decrypt` with.
- For key agreement use **ECDH** (P-256) to derive a shared AES-GCM key, or
  **RSA-OAEP** for key wrapping — both are in WebCrypto.
- Never persist raw key bytes in `localStorage`/`sessionStorage` (readable by any
  XSS — see [common-vulnerabilities.md](common-vulnerabilities.md)). If a key must
  survive a reload, store a non-extractable `CryptoKey` in IndexedDB, which
  structured-clones the handle without exposing the bytes.
- Rotate keys on a schedule and on any suspected compromise.

## When WebCrypto lacks the primitive: prefer WASM over JS

If you need something WebCrypto does not provide, a WASM build of a vetted crypto
library is preferable to a pure-JS one: a fixed WASM compilation target avoids the
JIT deoptimizations and GC pauses that make JS timing data-dependent, so it is
closer to constant-time and less exposed to timing side-channels. It is still not
a guarantee — keep secret-dependent branching and table lookups out of the code
regardless of language.
