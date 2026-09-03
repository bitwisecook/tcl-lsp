# KCS style guide

This is the full style guide for KCS notes in this repository. The short
summary lives in [`AGENTS.md`](../../AGENTS.md) under "Knowledge base and
documentation"; this file is the canonical source for examples and the
rules behind the rules.

A KCS note is a small, searchable answer to one question. It is written
in plain, simple English for a named audience. If you are tempted to
describe a contract, an interface, a data structure, or an architecture,
you are writing a design doc, not a KCS note. Put it under
[`docs/design/`](../design/README.md) instead.

## The six categories

A KCS note is always one of six types:

| Type | The question it answers | Template |
|---|---|---|
| **Issue** | Why is X not working, and how do I fix it? | [`kcs-template-issue.md`](templates/kcs-template-issue.md) |
| **Q&A** | What is X? / When should I use Y? | [`kcs-template-qa.md`](templates/kcs-template-qa.md) |
| **How-To** | How do I do X? | [`kcs-template-how-to.md`](templates/kcs-template-how-to.md) |
| **Functionality** | What does command/feature/tool X do, and how do I use it? | [`kcs-template-functionality.md`](templates/kcs-template-functionality.md) |
| **Diagnostic** | Per-code page for an E/W/S/T/IRULE diagnostic. | [`kcs-template-diagnostic.md`](templates/kcs-template-diagnostic.md) |
| **Optimisation** | Per-code page for an O-code optimiser rewrite. | [`kcs-template-optimisation.md`](templates/kcs-template-optimisation.md) |

If a note does not fit any of these six, it is probably a design doc.

## The rules

These rules are enforced by review. `AGENTS.md` and `CONTRIBUTING.md` link
here rather than restating them; cite rules by name, not number.

### 1. One note answers one question

A KCS note has one core question and one answer. If you find yourself
writing "there are three things to know here", split the note.

**Bad**

> ## Question
>
> How does the language server handle indexing, and what are the
> configuration options for the workspace scanner, and why does it
> sometimes miss files?

**Good**

Three notes:

- "How do I configure which folders the workspace scanner indexes?"
- "Why is my file missing from Find References?"
- "What does the workspace scanner do?"

### 2. Name the audience

Every KCS note starts with a blockquote header that names its audience
and type:

```markdown
# KCS: <title>

> **Audience:** User
> **Type:** Issue
```

The three audiences are:

- **User** — someone editing Tcl or iRules code in an editor. They do
  not read source files for tcl-lsp itself and do not know its internal
  names.
- **Contributor** — someone writing code for tcl-lsp. They understand
  Rust, the repo layout, and can read the compiler pipeline.
- **Maintainer** — someone with merge rights who makes cross-cutting
  decisions.

Pick one audience. Do not write for "users and contributors" in the same
note — the tone and vocabulary for each is different. Split the note
instead.

### 3. British English

Use British spelling throughout: `colour`, `behaviour`, `optimise`,
`analyse`, `normalise`, `serialise`, `recognise`. Common drift words:

| Wrong | Right |
|---|---|
| color | colour |
| behavior | behaviour |
| optimize | optimise |
| analyze | analyse |
| center | centre |
| initialization | initialisation |
| catalog | catalogue |
| license (verb) | licence (noun), license (verb) |

American spellings in the names of external products (`VS Code`, `Color
Theme Editor`) are fine — do not rename the product.

### 4. Oxford comma

Use the Oxford comma in lists of three or more:

**Bad**

> Highlights tokens, ranges and diagnostics.

**Good**

> Highlights tokens, ranges, and diagnostics.

### 5. Short, plain sentences

Aim for sentences of fifteen words or fewer. Avoid nested clauses.
Prefer verbs to noun phrases.

**Bad**

> The initialisation of the language server, which is triggered on the
> opening of the first Tcl file, will cause the package scanner to
> commence the enumeration of all source directories before the loading
> of any user-configured dialect stubs.

**Good**

> The language server starts when you open your first Tcl file. It
> scans every source directory in the project, then loads your dialect
> stubs.

### 6. No unlinked acronyms or specialist terms

On first use in a note, replace internal jargon with plain names. If a
technical term is unavoidable, link it to the
[glossary](../GLOSSARY.md).

**Bad**

> The LSP publishes diagnostics from the CFG/SSA stage after SCCP runs.

**Good**

> The language server publishes problem markers after the
> [compiler pipeline](../GLOSSARY.md#full-pipeline) finishes its
> [constant propagation](../GLOSSARY.md#sccp) pass.

"LSP" has a plain name — "the language server" — so it needs no link.
"CFG", "SSA", and "SCCP" do not, so they are either dropped or linked
to the glossary entry that defines them.

You do not need to expand every acronym — "VS Code" stays "VS Code",
"URL" stays "URL" — but anything internal to tcl-lsp (CFG, SSA, SCCP,
IR, compilation unit, SSA value key, lattice, shimmer, taint colour)
must either be replaced with a plain phrase or linked to the glossary.

### 7. Exact UI labels

When you point the reader at a button, menu, command palette entry, or
setting, use the exact text that appears on screen. Do not paraphrase.

**Bad**

> Restart the server from the command list.

**Good**

> Run **Tcl: Restart Language Server** from the Command Palette
> (`Ctrl+Shift+P` or `Cmd+Shift+P`).

### 8. No inline contracts

KCS notes never contain numbered "decision rules", "contracts", or
"ownership matrices". They never list file-path anchors. They never
paste a type signature. If the answer needs any of those, the answer
belongs in a design doc and the KCS note should link to it.

**Bad** — a KCS note with a "## Decision rules / contracts" section and
a list of twelve file paths.

**Good** — a KCS note that says "The core and LSP packages share a set
of position helpers. For the full contract, see [shared
utilities](../design/contracts/shared-utility-contracts-rust.md)."

The design docs are where a `## File-path anchors` section belongs, and
they already carry one. A KCS note that grows a second, drifting copy
gives the reader two lists to distrust instead of one to follow.

### 9. One screen

A KCS note should fit on one screen. If it is longer than eighty lines
of prose, consider whether it is actually two notes, or whether the
answer has grown into a design doc.

### 10. Name the file after the question, not the implementation

A KCS filename should describe what the note **answers** in the
reader's own words. The reader is searching for their problem, not
browsing a directory tree of your internal module names.

The shape is:

```
kcs-<type>-<short-question-in-plain-words>.md
```

where `<type>` is one of:

| Type prefix | Used for |
|---|---|
| `issue` | A user hits a problem and wants the fix. |
| `qa` | A single question with a short, plain answer. |
| `howto` | Task-oriented steps a user or contributor follows. |
| `feature` | A command, feature, or tool (Functionality notes). |
| `diagnostic` | A per-code page for an E/W/S/T/IRULE diagnostic. |
| `optimisation` | A per-code page for an O-code optimiser rewrite. |

**Bad**

- `kcs-issue-vscode-lsp-startup-logs.md` — named after the thing the
  answer *does* (look at a log channel), not the thing the user
  *experiences* (no squiggles, no hover, no completions).
- `kcs-issue-documentbuffer-offset-drift.md` — `DocumentBuffer` is an
  internal class name. A user does not know it exists.
- `kcs-howto-invoke-package-resolver.md` — `package resolver` is a
  module. The question is "how do I make tcl-lsp find my packages?".
- `kcs-diagnostic-w210.md` — the code alone is not a plain-English
  tail. A reader searching for "variable not used" will not find it.

**Good**

- `kcs-issue-lsp-features-are-missing.md`
- `kcs-issue-go-to-definition-jumps-to-wrong-line.md`
- `kcs-howto-make-tcl-lsp-find-my-packages.md`
- `kcs-qa-when-should-i-restart-the-server.md`
- `kcs-diagnostic-w210-variable-read-before-set.md`
- `kcs-optimisation-o105-constant-var-ref-propagation.md`

Functionality, diagnostic, and optimisation notes are named around
their stable identifier (the feature name or the code) because the
reader searches for them by that identifier: "what does O105 do?",
"why am I seeing W210?", "what is the rename feature?". The code or
feature name is the subject; the tail describes it in plain English.

**Filename casing** — diagnostic and optimisation code prefixes are
always **lowercase** in filenames: `kcs-diagnostic-w210-...`, not
`kcs-diagnostic-W210-...`. The code itself uses uppercase in prose
and headings (`W210`, `O105`) but the filename is all-lowercase for
consistency with the rest of the KCS tree and to avoid case-sensitivity
issues across platforms.

### 11. List the editors and tools the note applies to

Every KCS note must include an `## Applies to` section immediately
after the audience/type header. It is a **comma-separated plain-text
list**, not bullet points. Each item is a tag. The build and query
scripts normalise each tag by lowercasing it and replacing internal
spaces with a hyphen, so `VS Code` and `vs-code` are the same tag.

The tables below are the canonical tag vocabulary. `cargo xtask
kcs-index-links` checks every tagged per-code diagnostic page against the
controlled vocabulary, so a typo or obsolete stage tag fails the
documentation gate. The CLI
help builder normalises feature-note tags, and `LSP_EDITOR_TAGS` in
`rust/tcl-cli/build.rs` is the set `all-editors` expands to. Treat these
tables as the list of tags a reader can filter by.

#### Editor tags (driven by the LSP server)

| Tag | What it means |
|---|---|
| `vs-code` | VS Code |
| `zed` | Zed |
| `jetbrains` | JetBrains IDEs (IntelliJ, PyCharm, CLion, …) |
| `neovim` | Neovim |
| `helix` | Helix |
| `emacs` | Emacs |
| `sublime-text` | Sublime Text |
| `all-editors` | Shorthand for every editor above. The build script expands it to the full set before storing tags, so it never appears as a literal tag. |

#### Tool tags

| Tag | What it means |
|---|---|
| `tcl-lsp-cli` | The `tcl-lsp` command-line tool |
| `mcp` | The tcl-lsp MCP server (tools for AI agents) |
| `claude-skill` | Claude Code slash-command skills (`/irule-create`, `/tcl-fix`, …) |
| `copilot-chat` | VS Code Copilot Chat participants (`@tcl`, `@irule`, `@tk`) |

#### KCS type tags

These are derived automatically from the filename prefix
(`kcs-<type>-<name>.md`) by the build script, so you do not write
them in the `## Applies to` line — they are added for you and can
be queried alongside the editor and tool tags.

| Tag | What it means |
|---|---|
| `issue` | Issue note (problem + fix) |
| `qa` | Q&A note |
| `howto` | How-to note |
| `feature` | Functionality note (command, feature, or tool) |

#### Content tags

These tags describe what kind of thing a Functionality note
documents. Add them to the `## Applies to` line alongside the
editor and tool tags when they fit — they let readers and the help
tool filter by content kind (for example, "show me every
optimisation").

| Tag | What it means |
|---|---|
| `diagnostic` | An error, warning, security, taint, or iRule check (E, W, S, T, IRULE families). Also used as a type tag on per-code pages. |
| `warning` | A warning-severity check (subset of diagnostics) |
| `optimisation` | A rewrite performed by the optimiser (O-codes). Also used as a type tag on per-code pages. |
| `refactoring` | A manual code-action refactor the user triggers |
| `analyser` | A read-only analysis surface (hover, references, call hierarchy) |
| `transform` | A whole-document rewrite (format, minify, unminify) |

Add a content tag when the note is primarily about that kind of
thing. A page that documents O105 gets `optimisation`; a page
about call hierarchy gets `analyser`. Mixing tags is fine — the
minifier gets both `transform` and `tcl-lsp-cli`.

#### Compiler pass tags

Every per-code page and every feature page that directly consumes
compiler facts gets a compiler-pass tag in its `## Applies to` line,
naming the pass that produces the code or the facts the feature
reads. The tag is also the anchor the reader follows to the glossary
entry for the pass, which in turn links to the compiler design doc.

| Tag | What it means |
|---|---|
| `lexing` | Lexer and command segmenter (token stream, ranges) |
| `command-walk` | Registry-driven analyser walk over a segmented command (`args`, `arg_tokens`, and `CommandSpec` metadata) |
| `lowering` | IR lowering — source tokens to typed IR statements |
| `cfg` | Control-flow graph construction (basic blocks) |
| `ssa` | SSA construction (phi placement, version numbering) |
| `sccp` | Sparse conditional constant propagation |
| `liveness` | Live-variable analysis |
| `type-infer` | Type lattice inference over SSA |
| `gvn` | Global value numbering (redundancy elimination) |
| `cse` | Common subexpression elimination |
| `dce` | Dead-code elimination (unreachable, transitive, stores) |
| `licm` | Loop-invariant code motion |
| `instcombine` | Expression canonicalisation and strength reduction |
| `ipa` | Interprocedural analysis and `ProcSummary` |
| `memssa` | Memory-SSA, alias sets, versioned memory operations |
| `dataflow` | Def-use chains and the data-flow graph |
| `taint` | Taint source/sink propagation and colours |
| `shimmer` | Shimmer detection over the type lattice |
| `tail-call` | Tail-call and tail-recursion rewrites (O121–O123) |
| `code-sinking` | Assignment sinking into decision blocks (O125) |
| `unused-procs` | Unused iRules proc removal (O124) |
| `side-effects` | Structured side-effect classification |
| `exec-intent` | Command-substitution execution-intent classification |
| `rendered-props` | String content properties over SSA |
| `const-fold` | Compile-time constant folding |
| `strength-reduce` | Strength reduction (`x**2` → `x*x`) |
| `pattern` | Whole-idiom pattern recognition (`incr`, `end-N`, build-chain collapse) |
| `codegen` | Bytecode codegen, local variable table, peephole |

Every tag in this table matches a glossary entry in
[`docs/GLOSSARY.md`](../GLOSSARY.md). The glossary entry links to the
design doc under [`docs/design/compiler/`](../design/compiler/README.md).
When you add a compiler pass, update both in the same change: this
table, and the glossary entry it points at.

#### Writing the Applies to line

Write what the contributor adds to a KCS note in plain English:
`VS Code, Zed, JetBrains`. The parser handles the casing and the
hyphens. `all-editors` can be written as `all editors` or
`all-editors`; the parser expands it to the full LSP editor set
before storing tags.

**Bad**

> ## Applies to
>
> - VS Code
> - Zed
> - JetBrains

**Good**

> ## Applies to
>
> VS Code, Zed, JetBrains

**Good (all editors plus an AI tool)**

> ## Applies to
>
> all-editors, MCP

**Good (a feature note that documents a diagnostic)**

> ## Applies to
>
> all-editors, diagnostic

#### Adding a new tag

When you need a tag that is not already in the glossary above, add
it in the same change that introduces it. The steps are:

1. **Add a row** to the appropriate table in this file (rule 11)
   with a one-line description of what the tag means. Keep the
   table ordered alphabetically within its group, except for the
   compiler-pass table, which follows pipeline order.
2. **Update the vocabulary summary** in `AGENTS.md` (rule 12) so
   agents and reviewers see the full list at a glance.
3. **Update the diagnostic KCS tag gate** in
   `rust/xtask/src/kcs_index_links.rs` when the tag can appear on a
   per-code diagnostic page. Keep a diagnostic-stage tag tied to its
   emission owner, not a broad "analyser" label.
4. **If the tag names a new LSP editor**, add it to `LSP_EDITOR_TAGS`
   in `rust/tcl-cli/build.rs` as well, so `all-editors` expands to it
   and the help database groups the note correctly. No other tag
   needs a code change.

Tags are cheap — a new one costs two or three small edits. A fuzzy or
overloaded Applies to line is expensive — a reader cannot filter
by it cleanly. Prefer adding a new tag over reusing an existing
one with a stretched meaning.

### 12. Sub-headings when editors differ

If the steps or behaviour differ between the editors and tools listed
in `## Applies to`, split the answer into one sub-heading per editor
or tool inside the answer section (`## Answer` or `## How to use`).
Keep the sub-heading order the same as the `## Applies to` list.

**Bad** — a single wall of prose that tries to cover every editor at
once, with parenthetical notes like "(on Zed, use Ctrl+K)".

**Good**

```markdown
## How to use

### VS Code

Run **Tcl: Rename Symbol** from the Command Palette (`Ctrl+Shift+P`
or `Cmd+Shift+P`), or press `F2` on the symbol under the cursor.

### Zed

Place the cursor on the symbol and press `Ctrl+K` then `R`.

### Neovim

Run `:TclRename` on the symbol under the cursor.
```

When the steps are identical everywhere, do not add sub-headings —
write one paragraph under the answer section.

### 13. A fixed bug that needs no reader action is not a KCS note

A KCS note earns its place by changing what the reader does. If a bug is
fixed and the only thing anyone has to do is be on a current build, there
is nothing for a note to tell them — that belongs in the changelog and the
release notes, not here. Writing one anyway leaves the knowledge base
describing problems that no longer exist, which is worse than silence: a
reader who finds it cannot tell whether they are looking at a live fault.

Do not write (or keep) an Issue note whose whole answer is "this was
broken, it is fixed now, update".

Do write one when, even after the fix, the reader still has something to
do or something to know:

- **They must act.** A specific version range is affected and they need to
  recognise it, or the fix needs a restart, a setting change, a cache
  clear, or a migration step.
- **A boundary survives the fix.** The behaviour is deliberately still
  reported in nearby cases, or a residual gap remains. Say which, and why.
- **The symptom is a live diagnosis.** The same symptom has several
  possible causes and the note walks the reader through telling them apart.

The test to apply: strip out every sentence that only says the bug is
fixed. If nothing actionable is left, delete the note.

### 14. Functionality notes carry at least one concrete example

A note that describes a feature, command, or tool shows it: a before/after
code block for a transform, a code pointer showing where a diagnostic or
hover appears, or a screenshot of a visual panel. A description with no
example leaves the reader unsure whether they are looking at the right
feature.

## When to link the glossary

On first use in a note, link any term that is either

- internal to tcl-lsp (any term defined in `docs/GLOSSARY.md`), or
- common in compiler engineering but unfamiliar to a user (lattice, phi
  node, SSA, dominator).

Link syntax:

```markdown
the [control-flow graph](../GLOSSARY.md#cfg)
```

On second and later uses of the same term in the same note, you do not
need to re-link.

## When to split a note into a KCS note plus a design doc

If you start writing a KCS note and find yourself reaching for a
"decision rules" list, a file-path anchor list, or a data-structure
diagram, stop. Move the technical content into a new file under
[`docs/design/contracts/`](../design/) (or `docs/design/compiler/` for
compiler internals). Leave a short KCS note that answers a single
user-facing question and links to the design doc.

Example: the contract "every document gets a `DocumentBuffer` for
position lookups" is a design doc. The question "Why are my Go to
Definition positions off by one column?" is a KCS issue note, and it
links to the design doc for the underlying contract.

## Minimum quality bar

Before you merge a KCS note, check:

- [ ] It has a single core question.
- [ ] It has an `> **Audience:**` / `> **Type:**` blockquote header.
- [ ] It has an `## Applies to` section immediately after the header,
  written as a comma-separated plain-text list, not bullets.
- [ ] The filename describes the question in plain words, not an
  internal class or module name (see rule 10). Functionality notes
  are the exception and are named after the feature.
- [ ] It is in British English.
- [ ] It uses the Oxford comma.
- [ ] Every internal acronym or specialist term links to the glossary
  on first use.
- [ ] UI labels match what appears on screen.
- [ ] It does not contain a "decision rules", "contracts", or
  "file-path anchors" section.
- [ ] Where the answer differs per editor or tool, it uses
  sub-headings inside the answer section rather than inline asides.
- [ ] It is no longer than one screen of prose.
- [ ] Functionality notes include at least one concrete example
  (before/after code, a code pointer for an analyser, or a
  screenshot).
- [ ] It is linked from [`docs/kcs/README.md`](README.md).
