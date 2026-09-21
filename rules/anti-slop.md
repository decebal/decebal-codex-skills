# Anti-slop — write like an engineer reporting, not a model performing

Applies to everything a human reads: chat replies, commit bodies, PR and issue
text, runbooks, docs. Code comments have their own rules
([comments.md](comments.md)); these are compatible.

The failure this prevents is not bad grammar. It is **prose that performs
significance instead of carrying information** — the register that makes a
reader skim, and makes a real finding read like marketing.

## The incident

2026-09-16, reporting a live API key committed to a public repo:

> *"All verified, and the exposure is worse than a public repo file."*

Every word after "verified" is staging. The reader already knows a verification
happened; what they need is *what is exposed, since when, and whether it still
works*. The sentence delays all three to build a beat. The same report also
closed with an unrequested moral ("I'd suggest this gets written down as an
incident"), which is advice nobody asked for attached to a task in progress.

Cost: the user had to stop a live security escalation to say *stop writing like
this*. In a real incident, a reader learning to skim your openers is a safety
problem, not a style one.

## Never

**Escalation and reveal openers.** "All verified, and X is worse than Y." "It
failed — which is the whole point." "Here's the thing." "The short version."
"Let me be clear." "And that's when it got interesting." State the result, then
the evidence. No suspense; the reader is not an audience.

**The X-not-Y frame.** "This isn't a style issue, it's a safety issue." "Not
because X. Not because Y. But because Z." Anthropic's own prompting guide lists
this as a house tic, and [slopdetector](https://slopdetector.org/blog/signs-of-ai-writing)
puts the threshold at three repeats of the same contrast frame in one piece. One
is usually one too many. Say the thing that is true; drop the foil.

**Significance labels.** "Worth noting", "worth knowing", "the part that
matters", "the load-bearing one", "this is the important bit". If it did not
matter you would not have written it. Relabelling every point as important is
how a reader stops believing any of the labels.

**Compliment-then-correct.** Opening a correction or a peer reply with praise
("Good catch, but…", "Right call, though…"). Say the correction. Praise a
decision when praising it is the message, not as a cushion.

**The closing meta-lesson.** A final paragraph generalising the work into a
principle — "the same shape appears at three levels", "this is the fourth
instance today" — unless the user asked what the pattern was. Finish the report
and stop.

**Vocabulary that marks the register**: delve, tapestry, landscape, testament,
pivotal, vibrant, seamless, robust, leverage (as a verb), utilize, facilitate,
crucial, comprehensive, "deep dive", "game-changer", "in today's …".

**Sentence openers**: Certainly, Moreover, Additionally, Furthermore, Notably,
Importantly.

## Thresholds, so this is checkable rather than a vibe

- **Em dashes**: at most 2 per 100 words in prose. A full stop is almost always
  the better choice. (Community threshold: 20 per 1,000 words is the tell.)
- **Rule of three**: at most one polished triplet per 200 words. Three parallel
  clauses is a rhythm, and a rhythm read twice becomes a tell. Four items, or
  two, are fine — the list should be as long as the facts are.
- **Deletion test**: if a sentence can be deleted and no fact is lost, delete
  it. If more than a third of sentences survive that test, the piece is filler.
- **Restatement test**: every paragraph should yield a concrete fact a reader
  could repeat — a number, a path, a command, a decision, a consequence.
- **Emoji**: status markers only (check / stop / retry), in tables and logs.
  Never in prose, never as tone.

## Instead

- **Lead with the finding**, then the evidence that supports it, then the limit.
- **Numbers over adjectives**: "1,932 tests, 12.5s" not "comprehensive coverage";
  "public since 2026-03-27" not "long-standing exposure".
- **Name the thing plainly**: "the key still authenticates" beats "the exposure
  is live".
- **Vary sentence length because the content varies**, not to perform rhythm.
- **Say what you did not check.** A stated limit is worth more than a confident
  summary, and it is the one thing slop never contains.

## Sources

Patterns and thresholds cross-checked against
[slopdetector's 12 patterns](https://slopdetector.org/blog/signs-of-ai-writing),
the [anti-ai-slop-writing skill](https://github.com/jalaalrd/anti-ai-slop-writing)
(50+ words, 35+ phrases, 16 openers, 10 structural patterns), and
[Colin Gorrie on why LLMs write this way](https://www.deadlanguagesociety.com/p/rhetorical-analysis-ai).
Kept short deliberately: a banned-word list of 50 entries gets skimmed, and the
structural tics above are what actually make prose read as machine-written.
