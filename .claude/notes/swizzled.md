# Swizzled Astryx components

Astryx components copied into the project with `bunx astryx swizzle`, and
what Studio changed in them. The copies live in `src/components/astryx/`,
are kept as Astryx wrote them (Biome and markdownlint skip the folder; `tsc`
checks it), and every change of ours is marked with a `Studio:` comment.

| Component | From | Why |
| - | - | - |
| `CodeBlock` | `@astryxdesign/core` 0.7.3 | A wrapped line continues under its own indent (two more columns after a list marker), and words wrap at spaces, not mid-word; the collapsible header keeps the arrow cursor. Used for the statement (Markdown) and the record (YAML) in the Lesson details (owner, 23 October 2026). |

**Why:** Astryx's `CodeBlock` wraps with `word-break: break-all` and offers
no hook per line; Monaco would do it natively but weighs megabytes, and the
owner chose the swizzle.

**How to apply:** after an Astryx bump, swizzle the component again
(`bunx astryx swizzle CodeBlock --output src/components/astryx --overwrite`),
then carry over each `Studio:` change from `git diff`. Drop the copy when
Astryx gains the behaviour.
