# KCS: <CODE> — <plain-English question>

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, <compiler-pass-tag>

<!--
  - `all-editors` expands to the full LSP editor set.
  - `diagnostic` is the content tag. Write it out: the help database is
    built from `docs/kcs/features/` only, so nothing indexes a page in
    this directory and no tag is added for you.
  - `<compiler-pass-tag>` is the pass that produces this code. Pick the
    canonical tag from docs/kcs/STYLE.md rule 11 (for example `ssa`,
    `sccp`, `taint`, `cfg`, `ipa`, `lexing`, `lowering`, `type-infer`).
    If more than one pass contributes, list them all.
  - Nothing validates these tags, so an unrecognised one is indexed
    rather than rejected. Check each against the rule 11 tables.
-->

## Profiles

<Which profiles enable this code, as a comma-separated list. The
vocabulary in use is:

  - `default` — on for every document.
  - `opt-in` — off unless the user turns it on. Name the exact setting
    and its default here, and give one sentence on why it is off by
    default. The opt-in set is `DEFAULT_OFF_CODES` in
    `rust/tcl-lsp-server/src/lib.rs`; check the code is in it before
    writing `opt-in`.
  - `dialect:irule` / `dialect:tcl` — the code only fires for that
    dialect. Combine with `default`, as in `default, dialect:irule`.
  - `Reserved — see Status.` — the code is declared but never emitted.
    Use this when its row in `rust/tcl-core-types/src/diag_code.rs` is a
    `diag_reserved(...)` row, and add the `## Status` section below.

Verify the row before you write this line: `diag(...)` versus
`diag_internal(...)` versus `diag_reserved(...)` in
`rust/tcl-core-types/src/diag_code.rs` decides all three of the profile
line, the `## Status` section, and what `## How to suppress` may claim.>

## Question

<The question a user asks when they see this code in their editor,
written exactly as they would ask it. One sentence, no jargon.>

## Status

<Only for a `diag_reserved` code. Say plainly that the code is specified
in the diagnostic registry but is not emitted, that no editor or CLI
surface reports it, and that there is no `tclLsp.diagnostics.<CODE>`
setting to toggle — then say the page describes the check as designed,
for when it ships. Delete this section for every code that is actually
emitted.>

## Why

<Simple, plain-English explanation of why this check exists. Write it
as if explaining the real-world consequence to a colleague: "A missing
variable will cause a runtime error, stopping the Tcl script with an
error." One or two sentences. No jargon without a glossary link.>

## Symptoms

- <What the user sees first: the squiggle colour, the Problems panel
  message, the exact code text the analyser prints.>
- <Any follow-on symptoms that confirm the diagnosis.>

<A reserved code puts nothing on screen, so replace this section with
`## Intended message` carrying the message text from the code's row in
`rust/tcl-core-types/src/diag_code.rs`, quoted exactly.>

## Example that triggers it

```tcl
# A minimal self-contained snippet the analyser flags with this code.
# Keep it to one screen and use realistic names, not `foo`/`bar`.
```

The analyser reports **`<CODE>`** on <the specific token, range, or
line — say exactly what the reader sees>.

<For a reserved code, title this section "Example that would trigger it"
and write the sentence in the conditional — the analyser does not report
it today.>

## Fix

```tcl
# The same snippet rewritten so the analyser no longer flags it.
```

<One or two sentences on why the fix works. If there are multiple
valid fixes, show the one that is most idiomatic first and mention
the alternatives in prose.>

## How to suppress

<Say only what you have checked. The four scopes, smallest first:

  - **One command** — `# noqa: <CODE>` on its own comment line
    **immediately above** the offending command. Comments attach
    forward, so a trailing `;# noqa` at the end of the offending line
    does not silence that line, and a blank line between the comment
    and the command detaches the directive.
  - **One file** — a `# tcl-lsp: disable=<CODE>` directive in the
    leading comment block at the top of the file.
  - **One project** — `disabled = <CODE>` under `[diagnostics]` in
    `.tcl-lsp.ini` at the workspace root.
  - **One editor** — `tclLsp.diagnostics.<CODE>` set to `false`.
    **Verify this key exists before you name it.** Only codes with a
    generated per-code entry have one; search
    `editors/vscode/package.json` for `tclLsp.diagnostics.<CODE>` and
    check the code's row in `rust/tcl-core-types/src/diag_code.rs`. A
    setting that does not exist is the single most common defect on
    these pages.

Then the three cases that are not a plain "add a comment":

  - **Internal** (`diag_internal` in the code table) — say so: the code
    has no per-code entry in the generated editor settings list. Offer
    the file-level directive and the `.tcl-lsp.ini` key instead.
  - **Reserved** (`diag_reserved`) — there is nothing to suppress,
    because nothing is emitted. Say that, and point at `## Status`.
  - **Always on** — if the code genuinely cannot be turned off, say so
    and say why, rather than inventing a setting.

Close with a link to
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).>

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [<compiler-pass>](../../GLOSSARY.md#<pass-anchor>) — the pass that
  produces this code. `<pass-anchor>` is the glossary heading's own
  anchor, which is usually **not** the tag name: the `taint` tag's entry
  is `#taint-analysis`, `type-infer` is `#type-inference`, `const-fold`
  is `#constant-folding`. Open [the glossary](../../GLOSSARY.md), find
  the heading, and copy its anchor. Drop this line if the pass has no
  glossary entry.
- Related codes: `<CODE1>`, `<CODE2>`, `<CODE3>` — pick 2-3 codes in
  the same family or that a reader searching for `<CODE>` might
  also want.
