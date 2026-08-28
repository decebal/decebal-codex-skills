# The encrypted Envelope pattern

A single typed wrapper message carries every payload over the transport. It gives
you three things at once: a uniform frame to decode, a **sequence number** for
request/response correlation, and a place to apply **encryption** to the payload
without leaking the inner message type onto the wire.

## The Envelope proto

```proto
syntax = "proto3";
package envelope.v1;

message Envelope {
  uint64 sequence     = 1;   // monotonic; correlates a response to its request
  string payload_type = 2;   // logical type of the decrypted inner message
  bytes  iv           = 3;   // 12-byte (96-bit) AES-GCM nonce, fresh per message
  bytes  ciphertext   = 4;   // AES-256-GCM(inner message bytes); includes the 128-bit tag
}
```

The inner message is itself a protobuf message: serialize it with
`toBinary(InnerSchema, inner)`, encrypt those bytes, and place the result in
`ciphertext`. The recipient decrypts, then `fromBinary(InnerSchema, plaintext)`.
`payload_type` tells the recipient which schema to use.

## Encrypt at the envelope (AES-256-GCM)

Use WebCrypto. GCM is authenticated — a tampered ciphertext fails to decrypt
rather than returning garbage.

```ts
import { create, toBinary, fromBinary } from "@bufbuild/protobuf";
import { EnvelopeSchema } from "./gen/envelope_pb";

async function seal(key: CryptoKey, seq: bigint, type: string, innerBytes: Uint8Array) {
  const iv = crypto.getRandomValues(new Uint8Array(12));          // FRESH 96-bit IV per message
  const ct = await crypto.subtle.encrypt({ name: "AES-GCM", iv }, key, innerBytes);
  return toBinary(EnvelopeSchema, create(EnvelopeSchema, {
    sequence: seq, payloadType: type, iv, ciphertext: new Uint8Array(ct),
  }));
}

async function open(key: CryptoKey, frame: Uint8Array) {
  const env = fromBinary(EnvelopeSchema, frame);
  const pt = await crypto.subtle.decrypt({ name: "AES-GCM", iv: env.iv }, key, env.ciphertext);
  return { sequence: env.sequence, payloadType: env.payloadType, plaintext: new Uint8Array(pt) };
}
```

### The IV rule (do not get this wrong)

**Never reuse an (IV, key) pair.** GCM catastrophically fails on nonce reuse — an
attacker can recover the keystream and forge auth tags for that key. So:

- Exactly 12 random bytes from `crypto.getRandomValues`, generated for **every**
  message. A fixed or counter-from-zero IV in source is a critical bug.
- The IV is not secret — ship it in the envelope (`iv` field) next to the
  ciphertext. What must never repeat is the *pair*.
- Random 96-bit IVs are safe up to ~2^32 messages per key; past that, rotate the
  key. Prefer non-extractable `CryptoKey`s so XSS cannot read the raw bytes.

Full WebCrypto AES-GCM specifics, hashing, and key handling:
[../../security-review/SKILL.md](../../security-review/SKILL.md) and its
[encryption-patterns](../../security-review/references/encryption-patterns.md)
reference.

## Sequence numbers for correlation

Over a bidirectional stream (WebSocket), responses arrive out of order. A
monotonic `sequence` set by the sender lets the receiver match a reply to the
pending request:

```ts
let seq = 0n;
const pending = new Map<bigint, (r: DecodedInner) => void>();

function request(type: string, innerBytes: Uint8Array): Promise<DecodedInner> {
  const s = ++seq;
  return new Promise(async (resolve) => {
    pending.set(s, resolve);
    ws.send(await seal(key, s, type, innerBytes));       // binary frame
  });
}

ws.onmessage = async (e) => {
  const { sequence, plaintext, payloadType } = await open(key, new Uint8Array(e.data));
  pending.get(sequence)?.(decodeInner(payloadType, plaintext));
  pending.delete(sequence);
};
```

## Binary WebSocket frames

Set `ws.binaryType = "arraybuffer"` and send the `toBinary` output directly — a
binary frame, never `JSON.stringify` and never a text frame. `onmessage` yields an
`ArrayBuffer`; wrap it in `new Uint8Array(e.data)` before `fromBinary`. Use
`wss://` only ([../../security-review/SKILL.md](../../security-review/SKILL.md), H4).
