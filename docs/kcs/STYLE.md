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

## The four categories

A KCS note is always one of four types:

| Type | The question it answers | Template |
|---|---|---|
| **Issue** | Why is X not working, and how do I fix it? | [`kcs-template-issue.md`](templates/kcs-template-issue.md) |
| **Q&A** | What is X? / When should I use Y? | [`kcs-template-qa.md`](templates/kcs-template-qa.md) |
| **How-To** | How do I do X? | [`kcs-template-how-to.md`](templates/kcs-template-how-to.md) |
| **Functionality** | What does command/feature/tool X do, and how do I use it? | [`kcs-template-functionality.md`](templates/kcs-template-functionality.md) |

If a note does not fit any of these four, it is probably a design doc.

## The nine rules

These nine rules are enforced by review. The numbered list also appears
verbatim in `AGENTS.md` for quick reference.

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
  Python, the repo layout, and can read the compiler pipeline.
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

> The [language server](../GLOSSARY.md#lsp) publishes problem markers
> after the [compiler pipeline](../GLOSSARY.md#compiler-pipeline)
> finishes.

You do not need to expand every acronym — "VS Code" stays "VS Code",
"URL" stays "URL" — but anything internal to tcl-lsp (CFG, SSA, SCCP,
IR, CU, SSA value key, lattice, shimmer, taint colour) must either be
replaced with a plain phrase or linked to the glossary.

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
of position helpers. For the full contract, see [core/LSP shared
utilities](../design/contracts/core-lsp-shared-utility.md)."

### 9. One screen

A KCS note should fit on one screen. If it is longer than eighty lines
of prose, consider whether it is actually two notes, or whether the
answer has grown into a design doc.

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
- [ ] It is in British English.
- [ ] It uses the Oxford comma.
- [ ] Every internal acronym or specialist term links to the glossary
  on first use.
- [ ] UI labels match what appears on screen.
- [ ] It does not contain a "decision rules", "contracts", or
  "file-path anchors" section.
- [ ] It is no longer than one screen of prose.
- [ ] It is linked from [`docs/kcs/README.md`](README.md).
