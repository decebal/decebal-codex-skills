# Estimation — the effortHours incident

Detailed case study supporting `rules/estimation.md`.

## The incident, 2026-09-01

A workflow was authored to investigate three defects. Its output schema declared `effortHours` on every proposal and `correctedEffortHours` on every verdict, so eighteen agents were **required** to invent a number for work none of them had done. The verifier then "corrected" 6 to 14 and 1 to 2.5 — a fabrication refining a fabrication, in a currency that answers no question anybody had.

### What was actually useful

The useful finding in that same output was never a duration. It was *"this proposal is UNSAFE because the dedup is keyed on the value and moves the denominator"* — a statement about **confidence in the shape**. That is what the rock/sand/water scale exists to carry, and the hours actively crowded it out.

### The lesson

**Design the schema so the fiction is unrepresentable.** If a field must be filled and the information you actually need is confidence in understanding, use a confidence enum, not a duration.

When you inherit a schema with time fields, put the real answer — tier plus what would move it up — in the body where a human reads it. Never volunteer a duration a form did not force, and never propagate the field into anything you design yourself.
