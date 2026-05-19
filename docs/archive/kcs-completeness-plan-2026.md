# KCS completeness plan

## Context

The knowledge base (KCS) today covers 47 features, 11 top-level notes,
and a style guide. It has no per-code pages for the ~134 diagnostic,
warning, security, taint, iRule, and optimisation codes the analyser
and optimiser emit, and the compiler pass vocabulary the codes depend
on is only partly represented in the glossary. Contributors and users
who want to know "what is W210?" or "which pass produces O105?" have to
read source.

This plan lays out the work to take the KCS to 100% coverage of every
code and every compiler pass, with plain-English writing, consistent
tagging, and strong cross-linking. It is a living document: every
phase updates it, and the todo list in the PR stays in sync with the
section headings here.

## Goal

1. Every diagnostic, warning, security, taint, iRule, and optimisation
   code has its own KCS page under `docs/kcs/codes/`.
2. Every compiler pass that produces a code has a glossary entry that
   links to the underlying compiler design doc.
3. Every feature KCS note that surfaces a code links to the code page,
   and every code page links back to the feature that surfaces it and
   to the pass that produces it.
4. The tag vocabulary includes compiler-pass tags so readers can
   filter "show me every O-code that comes from GVN" or "every code
   produced by SSA construction".
5. Existing user-facing features that were missing a KCS page (19
   pages from the inventory) all get one.

## Success criteria

- `scripts/check/kcs_index_links.py` reports no broken links after
  every phase.
- `make prep-pr` is green after every commit.
- The KCS help database (`shared/help/kcs_help.db`) builds cleanly and
  every new page is reachable through `tcl_ai.py help`.
- Every code page passes the KCS minimum quality bar in
  [`docs/kcs/STYLE.md`](../kcs/STYLE.md).
- Every page in the plan has an entry in an index page; no orphans.

## Scope

### In scope

- All diagnostic codes produced by `analyser/` and `compiler/`,
  including E, W (style, security, variable), S, T, and every IRULExxxx
  family.
- All O-codes produced by `compiler/optimiser/`.
- Compiler-pass glossary entries for every pass listed in
  `compiler/` that produces or consumes a code.
- 19 missing feature pages from the inventory (high, medium, and
  low priority).
- Cross-linking between code pages, feature pages, the glossary, and
  the design docs under `docs/design/compiler/`.

### Out of scope

- Rewriting existing feature pages beyond adding Related-links in
  Phase 7.1.
- Rewriting existing contributor troubleshooting notes under
  `docs/kcs/` (kcs-issue-range-drift, kcs-issue-stale-compiler-cache,
  kcs-issue-duplicate-diagnostics, kcs-howto-ir-cfg-ssa-diagnostics).
  Those are tracked separately as a style-rule-8 cleanup.
- Changes to the diagnostic or optimiser implementation. This is
  documentation only.

## Directory layout

All code pages go in a single unified directory:

```
docs/kcs/codes/
├── README.md                         ← unified index, grouped by family
├── kcs-diagnostic-e001-missing-subcommand.md
├── kcs-diagnostic-w210-variable-read-before-set.md
├── kcs-diagnostic-irule4005-static-race.md
├── kcs-optimisation-o105-constant-var-ref-propagation.md
└── ...
```

Rationale for a single directory: contributors write one code page at
a time and want one template; readers searching for "o105" or "w210"
do not care whether it is a diagnostic or an optimisation; a single
`README.md` index is easier to keep current than two.

## Naming conventions

Code page filenames follow the shape:

```
kcs-<type>-<code>-<short-question-in-plain-words>.md
```

where `<type>` is `diagnostic` or `optimisation`, `<code>` is the
lowercased code identifier (`w210`, `o105`, `irule4005`), and the tail
is a plain-English description of the check or rewrite.

**Good**

- `kcs-diagnostic-w210-variable-read-before-set.md`
- `kcs-diagnostic-irule4005-static-variable-race.md`
- `kcs-optimisation-o105-constant-var-ref-propagation.md`

**Bad**

- `kcs-w210.md` — no human-readable tail; filename alone does not tell
  the reader what the code is about.
- `kcs-diagnostic-read-before-set.md` — missing code; stable code
  identifiers are how readers find the page from a diagnostic message.
- `kcs-optimisation-constant-propagation.md` — the O-code is the
  stable identifier the reader sees in hover text.

`diagnostic` and `optimisation` are added as two new KCS type
prefixes alongside `issue`, `qa`, `howto`, and `feature` in style
rule 10.

## Tag vocabulary additions

### Compiler pass tags

Every code page and every compiler-internals feature page gets a
compiler-pass tag in its `## Applies to` line. The canonical list
(added to `TAG_DISPLAY` in `shared/help/kcs_db.py` and to STYLE.md
rule 11):

| Tag | Display | What it is |
|---|---|---|
| `lexing` | Lexing | Token stream and command segmentation |
| `lowering` | IR lowering | Source to IR translation |
| `cfg` | CFG construction | Basic block decomposition |
| `ssa` | SSA construction | Phi placement and version numbering |
| `sccp` | SCCP | Sparse conditional constant propagation |
| `liveness` | Liveness | Live-variable analysis |
| `type-infer` | Type inference | Lattice-based type inference |
| `gvn` | GVN | Global value numbering |
| `cse` | CSE | Common subexpression elimination |
| `dce` | DCE | Dead-code elimination |
| `licm` | LICM | Loop-invariant code motion |
| `instcombine` | InstCombine | Expression canonicalisation |
| `ipa` | IPA | Interprocedural analysis (`ProcSummary`) |
| `memssa` | Memory-SSA | Versioned memory operations and alias sets |
| `dataflow` | Data-flow | Def-use chains and data-flow graph |
| `taint` | Taint | Taint source/sink propagation |
| `shimmer` | Shimmer | Shimmer type-lattice detection |
| `tail-call` | Tail-call | Tail-call and tail-recursion rewrites |
| `code-sinking` | Code sinking | Assignment sinking into decision blocks |
| `unused-procs` | Unused procs | Unreferenced proc removal |
| `side-effects` | Side-effects | Structured side-effect classification |
| `exec-intent` | Execution intent | Command-substitution classification |
| `rendered-props` | Rendered properties | String content properties over SSA |
| `const-fold` | Constant folding | Compile-time folding of constants |
| `strength-reduce` | Strength reduction | `x**2` → `x*x`, `x%8` → `x&7` |
| `codegen` | Codegen | Bytecode lowering, LVT, peephole |

These tags are **informational** (they categorise the page); they do
not change the `Applies to` category for runtime grouping.

### KCS type prefix additions

`diagnostic` and `optimisation` are added to the KCS type tag set so
the build script can auto-tag code pages from their filename prefix:

| Tag | Display |
|---|---|
| `diagnostic` | Diagnostic |
| `optimisation` | Optimisation |

(`diagnostic` already exists as a **content tag**; the build script
needs to handle the type tag and the content tag as the same key, or
the content tag gets renamed. Decision: keep `diagnostic` as both a
content tag and a type tag; they mean the same thing so there is no
conflict.)

## Page templates

### Code page template (diagnostic)

```markdown
# KCS: <code> — <plain-English question>

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, <compiler-pass-tag>, diagnostic, <severity-tag>

## Profiles

<Which diagnostic profiles enable this code and which suppress it —
the default profile, the strict profile, the optimiser profile, etc.
If the code is opt-in, say so here and give the setting name.>

## Question

<The question a user asks when they see this code in their editor,
written exactly as they would ask it.>

## Symptoms

- <What the user sees: the squiggle colour, the Problems panel
  message, the exact code text.>

## Example that triggers it

```tcl
# A minimal snippet the analyser flags with this code.
```

## Why it matters

<Plain-English explanation of why this is worth fixing — the risk,
the bug, or the style concern. No jargon without a glossary link.>

## Fix

```tcl
# The same snippet rewritten to silence the diagnostic.
```

<One or two sentences on why the fix works.>

## How to suppress

<Inline suppression (`# noqa: W210`), per-file, and settings-level
toggle if one exists.>

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [<compiler-pass>](../../GLOSSARY.md#<anchor>) — the pass that
  produces this code
- Related codes: `W211`, `W220`, …
```

### Code page template (optimisation)

```markdown
# KCS: <code> — <plain-English question>

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, <compiler-pass-tag>, optimisation

## Profiles

<Which optimiser profiles enable this rewrite: `readability`,
`standard`, `full`. If the optimisation is always-on, say so.>

## Question

What does <code> rewrite, and when does it fire?

## Before

```tcl
# Input source the optimiser sees.
```

## After

```tcl
# The rewrite the optimiser produces.
```

## Why

<Plain-English reason the rewrite is a win: correctness, performance,
readability, or all three.>

## Safety conditions

<When this optimisation is skipped — for example, when a variable is
shimmered, when a barrier is present, when taint colours require
sanitisation.>

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [<compiler-pass>](../../GLOSSARY.md#<anchor>) — the pass that
  produces this rewrite
- Related codes: `O100`, `O102`, …
```

### Feature page template

Unchanged from [`docs/kcs/templates/kcs-template-functionality.md`](../kcs/templates/kcs-template-functionality.md).
New feature pages in Phase 2 and Phase 3 follow it verbatim.

## Cross-linking rules

Every page in this plan belongs to a web of links. The rules below
define the minimum a page must include to be considered complete.

### Code page

- **Back** to `docs/kcs/codes/README.md` (the index).
- **Up** to the feature page that surfaces the code
  (diagnostics feature for most; optimiser feature for O-codes;
  formatter feature for W111/W112/W118; etc.).
- **Sideways** to 2-3 related codes in the same family.
- **Glossary** to the compiler-pass entry that produces the code.
- **Design doc** via the glossary — code pages do not link directly
  to `docs/design/compiler/` files; the glossary is the single hop.

### Feature page

- **Back** to `docs/kcs/features/README.md`.
- **Down** to every code page it surfaces (full list).
- **Sideways** to 2-3 related features.
- **Glossary** for any internal term.

### Glossary entry

- **See also:** link to the `docs/design/compiler/` design doc for
  the pass.
- **Cross-links** to 1-2 related glossary entries.
- (Glossary entries do not link to code pages — the direction is
  code → glossary, not the other way, or the glossary would become a
  reverse index.)

### Index pages

- `docs/kcs/codes/README.md` — groups every code page by family (E,
  W, S, T, IRULE, O), with the plain-English tail of each filename
  visible in the link text.
- `docs/kcs/README.md` — links to `codes/README.md` and the feature
  index, with a one-line count of how many pages each contains.

## Profile handling

Every code page has a `## Profiles` section that lists which analysis
or optimiser profile enables the code. The vocabulary:

- **Diagnostic profiles**: `default` (on by default), `opt-in` (off
  unless the user enables the code), `strict` (on in a stricter
  profile), `dialect:irule` (on for iRules only), `dialect:tcl` (Tcl
  only). Multiple profiles may apply.
- **Optimiser profiles**: `readability`, `standard`, `full`. Most
  O-codes are always-on within the selected profile.

When a code is opt-in (today only `W123`), the page names the exact
setting and its default value. Per-code pages do not get their own
"readability profile" page; the profile column on the O-code table in
the codes index gives the same information at a glance.

## Phase plan

See the todo list in this PR for the current state. High-level:

| Phase | What | Commits |
|---|---|---|
| 0 | Foundation — plan, tag vocab, templates, index scaffolding | 5 |
| 1 | Glossary backfill — 21 pass entries + design-doc cross-links | 3 |
| 2 | High-priority feature pages (9) | 3 |
| 3 | Medium + low-priority feature pages (10) | 2 |
| 4 | E/W/S/T code pages (~55) | 4 |
| 5 | IRULE code pages (32) | 4 |
| 6 | O code pages (28) | 4 |
| 7 | Cross-linking sweeps + final validation | 4 |

Each commit must pass `make prep-pr`. Each phase ends with a commit
and a push to the existing PR branch; no new PRs.

## Quality bar for each new page

Before a new page is considered done in this plan, it passes:

- [ ] The KCS minimum quality bar in `docs/kcs/STYLE.md` (Applies to,
  British English, Oxford comma, one question, one screen, plain
  English, sub-headings when editors differ).
- [ ] Code page template filled in end-to-end; no `TODO` or
  placeholder text left in the file.
- [ ] At least one concrete code example (triggering snippet for
  diagnostics, before/after for optimisations, screenshot reference
  or snippet for features).
- [ ] Every internal term either plain-English or linked to the
  glossary on first use.
- [ ] Compiler-pass tag present in `## Applies to`.
- [ ] Profile list present (code pages only).
- [ ] Cross-links in place per the rules above; no orphans.
- [ ] Linked from the relevant index page
  (`codes/README.md`, `features/README.md`, or `docs/kcs/README.md`).
- [ ] `scripts/check/kcs_index_links.py` passes.
- [ ] `make prep-pr` passes.

## Open items

- **Opt-in codes beyond W123.** The inventory flagged only `W123` as
  opt-in today. If Phase 4 uncovers more, add a shared
  `kcs-howto-enabling-opt-in-checks.md` rather than a per-code
  settings section.
- **Codes without a stable identifier.** A handful of internal codes
  (T103, T106) are propagation-only and do not surface to users. The
  codes index notes them in a short "internal codes" section rather
  than giving each a full page.
- **Optimisation profiles.** `readability` / `standard` / `full` is
  the vocabulary the optimiser uses today. If the profile model
  changes (for example, per-project profiles), the `## Profiles`
  section on every O-code page updates alongside.
- **Existing feature pages missing the audience blockquote.** All 46
  pages under `docs/kcs/features/` currently lack the
  `> **Audience:**` header required by style rule 2. This is tracked
  by the link checker as a warning (not a failure). Phase 7.1 adds
  the header alongside the Related-link sweep so both land in the
  same commit.
