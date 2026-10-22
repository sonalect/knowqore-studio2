# Signal where Knowqore failed you

This repository is also a live test of Knowqore. Work as usual and follow
`knowqore-consider.md`. This file adds one duty, and the owner counts it as
important: `dogfood/` is your folder for everything about Knowqore that you
dislike or that looks strange. Concrete cases are the most valuable thing
in it: the owner and you take them apart later, down to the data, to find
out why they happened. When in doubt, write it down; do not hold back.

Append one line to `dogfood/signals.md` (create the file when it is missing;
git does not track it) when:

- a lead misled you: you followed it, or nearly did, and it was wrong or
  outdated (amend it as the rule says, and note it here too);
- a call to `consider`, `assess`, `respond`, `record`, or `amend` failed or
  was refused;
- you had to find something yourself in the owner's other repositories, or
  the human had to tell you, that Knowqore should have offered for this task;
- a lead's `address` (or a source its statement names) is wrong, missing, or
  points to another file than the one that holds what it describes;
- a lead buries what answers the task inside unrelated material, so you had
  to open the source to find the one useful part;
- anything else that looks wrong, strange, or unhelpful: an answer you did
  not expect for that intent, the same unrelated lead coming back again, a
  lead whose kind, conditions, status, or date do not fit what it says, a
  rule of the workflow that got in your way.

One line each, in this form:

```text
YYYY-MM-DD HH:MM | <response_key, or -> | <misled|call|miss|address|noise|odd> | <what, in one sentence>
```

Make the line usable without this session: name the lead (`L1`), what you
asked (the `intent`, short), and what you expected instead. A case that
needs more than a line goes into its own file in `dogfood/`, named by date
and topic, with the line pointing to it.

Do not:

- change a verdict, a proposal, a choice, a record, or an amendment because
  of this file;
- add a block about this file to your reply unless the human asks;
- note a lead only because it was off topic once: your verdicts already
  hold that. Note it when it keeps coming back, or when it was off topic in
  a way that looks like a bug.
