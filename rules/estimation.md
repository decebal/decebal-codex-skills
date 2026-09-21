# Estimation — the betting table, rock/sand/water, never time

Portable. No stack assumptions.

## Never estimate in time. Ever.

No hours, no days, no weeks, no sprints, no "a quick one", no "an afternoon".
Not in reports, not in PR bodies, not in proposals, not in commit messages, not
in prompts to a subagent, and **not as a field in a schema you design**.

A time estimate is a forecast dressed as a fact. It invites the reader to plan
against a number nobody measured, and once it is written down it gets treated as
a commitment rather than the guess it was.

**Measurements are not estimates and stay welcome.** "cold clippy took 3m36s",
"the suite ran in 22s alone and could not finish inside 300s under load", "the
gate is capped at 300s" — those are facts with a stopwatch behind them. The ban
is on predicting the future in time units, not on reporting the past.

| Banned (a forecast) | Correct |
|---|---|
| "roughly one afternoon" | `rock` — bet it |
| "~4h", "~2 days", "a sprint" | `sand` — shape it first |
| `effortHours: 4` in a schema | `confidence: "rock" \| "sand" \| "water"` |
| `correctedEffortHours: 14` | `confidence` + what would move it up |

### The incident

On 2026-09-01 a workflow was authored to investigate three defects. Its output
schema declared `effortHours` on every proposal and `correctedEffortHours` on
every verdict, so eighteen agents were **required** to invent a number for work
none of them had done. The verifier then "corrected" 6 to 14 and 1 to 2.5 — a
fabrication refining a fabrication, in a currency that answers no question
anybody had.

The useful finding in that same output was never a duration. It was *"this
proposal is UNSAFE because the dedup is keyed on the value and moves the
denominator"* — a statement about **confidence in the shape**. That is what the
scale below exists to carry, and the hours actively crowded it out.

Design the schema so the fiction is unrepresentable.

## Rock / sand / water — confidence, not size

The scale measures **how well the work is understood**, not how big it is. A
huge, well-understood migration is a rock. A one-line change nobody can predict
the blast radius of is water.

| | The state | What it licenses |
|---|---|---|
| **rock** | Solid. Understood well enough to bet. The shape is known and the unknowns are named. | **Bet it.** Put it on the table. |
| **sand** | Shifts under you. Scope is not yet trustworthy — it will move once you start. | **Shape it first.** Do not bet sand; it is how a bet silently becomes three. |
| **water** | Formless. Unmapped. Cannot be sized at all yet. | **Spike it first.** The output of the spike is a classification, not a solution. |

Because the scale names a state, it always implies a **next action** — which is
what makes it more useful than a number. When asked "how long will this take",
answer with the classification, the reason, and **what would move it up a
tier**:

> `sand`. The gate's scoring model decides the verdict and the seam has never
> been measured under the consequential posture. Giving the harness a
> consequential arm and a captured corpus makes it a rock.

Never smuggle time back in through the tier ("a rock is about a week"). A tier
is not a duration in disguise; two rocks routinely differ by an order of
magnitude in effort and that is fine, because the tier is a claim about
certainty.

## The betting table

Work is **bet on**, not scheduled.

- **Only rocks reach the table.** Sand gets shaped, water gets a spike, and
  those are themselves bets — a shaping bet and a spike bet are legitimate
  things to commit to, and they are the correct answer to "we don't know yet."
- **A bet is a commitment to a shaped piece**, made deliberately, with the right
  to say no. Nothing arrives on the table by accumulating.
- **No backlog.** Unbet work does not queue up and does not rot in a list
  waiting to be re-estimated. If it matters it gets bet on next time; if it never
  gets bet on, that was the answer.
- **No rollover by default.** A bet that did not land is re-bet on its merits,
  not automatically continued because it was "nearly done" — a claim that is
  itself usually a time estimate wearing a disguise.

## When someone else's format demands a number

An external tracker, a client template, or a tool schema you do not control may
have a duration field. Fill it if it is mandatory, and put the real answer —
tier plus what would move it up — in the body where a human reads it. Never
volunteer a duration a form did not force, and never propagate the field into
anything you design yourself.
