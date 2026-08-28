# Fix it in the data, not in the binary

Applies to any product with a **generic engine** plus a **per-tenant data plane** —
published prompts/skills, config documents, rule packs, workflow definitions,
connector mappings. The engine and the connectors are a harness. What a given
customer does — what to classify, what to write, which fields a partner API accepts
— is **data**.

This rule governs both where new behaviour goes and, more often missed, where a
DEFECT gets repaired.

## The fix-site rule

When a customer-facing output is wrong there are two possible repair sites, and the
binary is almost always the wrong one. **Default to publishing a new data version.**
Data is instant, per-tenant, and it teaches the model the rule once. A patch in the
connector is one field, on one tenant's path, in something that has to be built,
shipped and installed.

The measured case. A partner API rejected `Online` where it wants `online`, and the
instinct was `.to_lowercase()` in the connector. But the published mapping said
`onlinestatus ← payload.online_status` — a **pass-through** — and nothing in the
chain ever stated the acceptable values. Normalising in the binary repairs that one
field while the model still does not know the contract, so the two sibling fields
declared as bare `"type": "string"` fail identically and each needs its own shipped
patch.

Of the five bugs found that day, **four were data fixes and one was
infrastructure. None were app code.**

## Triage order — in sequence, before proposing any fix

1. **Read the published manifest first** and find the field in its mapping.
   `X ← payload.y` is a pass-through: the defect entered UPSTREAM and the fix
   belongs upstream too. A value that is correct where it is created stays correct
   through every consumer — repairing it at the last hop leaves every other
   consumer broken.
2. **Check whether the data already says the right thing.** It often does. One
   manifest already carried *"treat a 422 duplicate as ALREADY UPLOADED … do NOT
   retry as a new create"*, and the runtime violated it twice in 80 seconds. A
   contract that exists and is disobeyed is a **different bug** from a missing
   contract: strengthen the wording, or look ABOVE it for an engine-level retry —
   never write the rule a second time in the binary.
3. **Confirm the partner's actual behaviour before calling it a bug.** The same
   session nearly filed a documented append-on-update as data corruption. Only the
   note's CONTENT was wrong.
4. **Only then** consider an app change, against the two cases below.

## What earns an app change

- **A generic engine capability with no customer name in it.** "A batch fold never
  attributes one item's output to another" is generic. "Gate the `<customer>_*`
  tools" is not — and it usually duplicates a knob that already exists (mark the
  step `requires_approval` in the data and publish). A PR doing exactly that was
  reverted for this reason.
- **Infrastructure the data cannot reach at all** — a secret left staged with no
  restart, a credential, a deploy.

## Declaring a contract ≠ correcting output

Enumerating an API's allowed values in a tool schema is fine — it **tells** the
model, and the next author sees it. Silently rewriting what the model emitted is
not: it masks data still producing wrong output, and the mask is invisible in every
log and every test.

**Never add a masking guard to a connector.** A sanitiser stripping escape residue,
or a check comparing a note's text to its URL, hides the real defect on one tenant's
path — and in a submit lane it silently alters human-authored text that the lane
exists to pass through verbatim.

## Where things live

| Concern | Home |
|---|---|
| What to extract, classify, or write; field mappings; partner enum values | published data |
| Pause-for-review before a write | a flag on the step, in the data |
| Which tools/connections a step gets | the step's frontmatter |
| Generic engine mechanics (retry, batch fold, grounding gate) | app code, no customer names |
| Secrets, deploys, machine state | infrastructure config |
