# Data over binary — the measured case

Detailed incident supporting `rules/data-over-binary.md`.

## The measured case

A partner API rejected `Online` where it wants `online`, and the instinct was `.to_lowercase()` in the connector. But the published mapping said `onlinestatus ← payload.online_status` — a **pass-through** — and nothing in the chain ever stated the acceptable values. 

Normalising in the binary repairs that one field while the model still does not know the contract, so the two sibling fields declared as bare `"type": "string"` fail identically and each needs its own shipped patch.

**Of the five bugs found that day, four were data fixes and one was infrastructure. None were app code.**

This illustrates why the triage order matters: read the published manifest first, check whether the data already says the right thing, and only then consider a binary change. The binary is almost always the wrong repair site.
