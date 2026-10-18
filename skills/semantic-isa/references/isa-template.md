# Semantic ISA specification template

Place the completed contract in the target repository's indexed `docs/` layout.
Adapt the structure to the system; mark genuinely inapplicable sections with a
reason. Do not fill unknown authority or runtime facts with proposed defaults.

## Scope and side effects

Name the agent, complete user job, environment and ranked harms. Separate
existing capabilities from proposals and unavailable integrations.

| Capability | System/resource | Data read or disclosed | Effect/cost | Current authority | Instruction |
| --- | --- | --- | --- | --- | --- |
| Example: change a private goal | Workspace aggregate | Goal fields | Durable edit | Narrow workspace write grant | `goal.revise` |

Every reachable capability maps to a binding or is removed from the agent's
reachable tool set. List out-of-bound surfaces explicitly; the contract does
not govern the entire host merely because it governs one agent.

## Instruction registry

| Instruction | Core | Governance property | Input/output refs | Authority | Sink | Recovery |
| --- | --- | --- | --- | --- | --- | --- |
| `goal.propose` | Cognitive | Probabilistic proposal | Proposal schema | Permitted context | No mutation; disclosure checked | Reject invalid proposal |
| `goal.read` | Memory | Authorised I/O | Goal reference / projection | Read grant | Private output | Redact or refuse |
| `VERIFY(goal_revision)` | Normative | Deterministic checkpoint | Captured revision / receipt | Kernel-only | No external effect | Conflict result |
| `goal.revise` | Execution | Validated domain write | Revision patch / receipt | Write grant | Durable mutation | CAS conflict/reconcile |
| `progress.assess` | Meta-cognitive | Advisory evaluation | Evidence refs / assessment | Permitted context | No authority change | Unknown result |

Properties distinguish probabilistic proposals, deterministic transforms,
authorised I/O, effectful actions, trusted checks and advisory routing. “An
external action is admitted deterministically” does not mean its result is
deterministic. Give domain instructions finer names than `TOOL_CALL`.

## Typed bindings

Define every input and output, including errors. Closed objects, explicit units,
bounded arrays/strings, enumerated variants and nullable unknowns are required
where they prevent ambiguity. Declare schema and ISA version compatibility.

Example binding, deliberately limited to changing a goal's name:

```json
{
  "instruction": "goal.rename",
  "input_schema": {
    "type": "object",
    "additionalProperties": false,
    "required": ["operation_id", "goal_id", "expected_revision", "name"],
    "properties": {
      "operation_id": { "type": "string", "minLength": 1, "maxLength": 128 },
      "goal_id": { "type": "string", "minLength": 1, "maxLength": 128 },
      "expected_revision": { "type": "integer", "minimum": 1 },
      "name": { "type": "string", "minLength": 1, "maxLength": 200 }
    }
  },
  "output_schema": {
    "type": "object",
    "additionalProperties": false,
    "required": ["status", "operation_id", "revision"],
    "properties": {
      "status": { "enum": ["committed", "refused", "uncertain"] },
      "operation_id": { "type": "string", "minLength": 1, "maxLength": 128 },
      "revision": { "type": ["integer", "null"], "minimum": 1 }
    }
  }
}
```

This schema is not a complete authorisation policy. Bind the principal and
workspace server-side; reject unknown fields, inaccessible resources, blank
normalized names and stale revisions. A model cannot submit `actor` or `approved`.
Define precise error details and their disclosure policy in the actual binding.

## Provenance, checks and sinks

Track at least source lineage, confidentiality/context grant, integrity/review
state and actor authority. Label model outputs and retrieved content. Propagate
labels through transformation, persistence, caching, export and delegation.

| Sink | Required verification | Bound inputs | What remains unproved |
| --- | --- | --- | --- |
| Private goal write | Membership, field grant, schema and revision | Actor/client, workspace, patch digest, revision, policy | Truth of any user-supplied market assumption |
| Paid data request | Permitted purpose and durable budget reservation | Query digest, provider, scope, occurrence, max cost | Availability or factual accuracy of returned data |
| Outbound message | User authority, exact recipient/content/destination | Message digest and current grant | Delivery until provider receipt/status confirms it |

A verification receipt includes check ID/version, input digest, principal,
resource/destination, source/policy revisions, as-of time and expiry. It covers
only those inputs. Failed verification preserves labels; successful verification
never strips provenance or privacy. Recheck volatile authority at execution.

## Admission and execution

Specify the pure decision as `admit(instruction, inputs, registry_snapshot,
as_of) -> permit | refuse(rule, evidence)`. No network, ambient clock, model or
filesystem call inside that predicate. Unknown evidence cannot pass a required
predicate. Reject unregistered instructions rather than fuzzy matching them.

Execution acquires current state, checks admission, reserves resources, performs
the effect and persists/reconciles its receipt. Describe the TOCTOU protection:
CAS, exact review binding, destination idempotency or another real mechanism.
Record operation IDs and replay semantics; distinguish a durable local commit
from uncertain remote execution. Do not promise exactly-once side effects
without a provider contract that supplies them.

## Violations and recovery

| Violation | Safe evidence | Recovery |
| --- | --- | --- |
| Unsupported instruction/schema | Instruction/field, redacted reason | Correct proposal within bounds; no guessed conversion |
| Missing authority/context grant | Required permission, no foreign existence leak | User grants access through trusted flow or narrow task |
| Stale review/revision | Authorised revision/diff | Re-prepare; never overwrite silently |
| Missing verification at sink | Dependency path and failed predicate | Collect missing evidence or refuse |
| Uncertain external effect | Stable operation ID and provider status | Reconcile before retry; retain paid reservation |
| Storage failure | Stage/commit receipt state | Resume same operation; no fabricated success |
| Budget/deadline exceeded | Consumed/reserved budget and partial coverage | Stop new work; retain partial results honestly |

Set bounded recovery, deadlines, cancellation and terminal refusal. Failures
may be recoverable without permitting an unbounded prompt/retry loop.

## Architecture and sequence

Draw model, typed binding, pure admission, registry, executor, store and external
environment. Include response/context disclosure and any delegated agent.
Sequence one failed instruction, its check, then an allowed or refused retry.
Show the commit-time authority check and final receipt/readback.

## Test matrix and rollout

Use scripted proposals and deterministic fixtures, with the actual adapter
boundary exercised where effects occur.

| Fixture | Required result |
| --- | --- |
| Every sink with missing required proof | Refused before effect; named failed predicate |
| Same sink with correctly bound proof | Only the authorised effect occurs |
| Schema-only proof presented as user authority | Refused |
| Changed payload/resource/client after verification | Old receipt rejected |
| Revoked membership between prepare and commit | Refused |
| Provenance stored/loaded/rendered/delegated | Labels survive; no trust upgrade |
| Duplicate operation, same payload | Same receipt, no duplicate local effect |
| Duplicate operation, different payload | Conflict |
| Lost response after possible external effect | Status/reconciliation before retry |
| Deadline/budget/partial source failure | Bounded stop and honest coverage |
| Hostile instructions inside source data | No change in capabilities or policy |
| Same snapshot/instruction/as-of replayed | Same admission result |

Shadow evaluations use fixtures or retain existing enforcement. List what the
new kernel does not yet govern. Activation needs real executor proof and user
authority; a written spec, installed skill or successful schema check is not
activation evidence.
