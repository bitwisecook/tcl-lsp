---
name: spec-author
description: "Build command specs for a private Tcl library: scan its sources with the compiler, infer each command's signature and behaviour, and produce stub sidecars or registry-ready spec drafts. Use when a user's own package commands are unknown to tcl-lsp, when annotating a private library, or when preparing a command-spec contribution."
allowed-tools: mcp__tcl-lsp__read_proc_docs, mcp__tcl-lsp__analyze, mcp__tcl-lsp__command_info, mcp__tcl-lsp__spectcl_check, mcp__tcl-lsp__spec_import, Read, Write, Glob, Grep, Bash
---

# Spec Author

Turn a private Tcl library into command specs the tools understand, from the
compiler's own inference — never guesswork from names.

## Read first

- `docs/design/spec-dsl-examples/README.md` — the frozen SpecTcl syntax,
  the output format for everything here; study the eleven `*.tclspec`
  ports beside it before writing a line.
- `docs/kcs/kcs-howto-create-command-specs-without-rust.md` — the workflow
  and where each artefact goes.
- `docs/design/compiler/command-registry.md` — every spec field.
- `docs/design/contracts/proc-arg-traits.md` — inferred parameter traits →
  argument roles and trait bits.
- `docs/kcs/kcs-howto-annotate-commands-with-stubs.md` — stub syntax and
  limits (no subcommands, no arity, sidecar `<dialect>.tcl.stubs`).

## Steps

1. From `$ARGUMENTS` or by asking: the source directory and the target —
   **private** (a SpecTcl pack that stays in the project) or
   **contribution** (the same pack plus a GitHub issue body).
2. Glob the `.tcl` files; note `package provide` (→ `required_package`) and
   `namespace export` (→ the public surface).
3. Per file, call `mcp__tcl-lsp__read_proc_docs` with the contents: every
   proc with params, parsed docstring, and inferred param traits — the same
   inference the Spec Studio uses.
4. Derive each spec from evidence, citing it: arity from the parameter list
   (defaults optional, trailing `args` variadic); roles and trait bits from
   the traits per the proc-arg-traits contract; hover text from the
   docstring; `required_package` from `package provide`. A parameter with
   no traits is a plain `Value`. Claim only what the evidence shows.
5. Check each name with `mcp__tcl-lsp__command_info`: a clash with a
   built-in needs the user's decision, never a silent overwrite.
6. Write **SpecTcl** either way: one `<library>.tclspec` at the library root
   declaring `speclib <name> 2.0`, in the frozen syntax (schema keys as
   property words, catalogue spellings verbatim, hook bodies only where the
   evidence demands one and always in the family's calling convention).
   Workspace discovery loads it — no stub sidecar needed. A contribution
   adds the issue body per the how-to; the Spec Studio renders the `.rs`.
   Validate with `mcp__tcl-lsp__spectcl_check` (pack source + target
   `dialect`): it loads through the real loader and reports per-command
   draft fields, every loader **notice** (`line`, `context`, `reason` —
   each a dropped word, so a typo'd trait shows up here and nowhere else),
   every **hook** with family and cacheability, and every **collision**
   with a shipped name. Fix every notice and every `shipped-spec-wins`
   collision (rename, or `-override` only when replacing the shipped spec
   is intended). An uncacheable hook is a cost to report, not a finding.
   Without the tool, self-check against the syntax memo's coverage matrix
   and say validation was manual.
7. Re-run `mcp__tcl-lsp__analyze` on a library file that *uses* the
   commands: the unknown-command diagnostics are gone (private) or you list
   what clears once the specs ship (contribution).
8. Report: commands covered, evidence per inference, anything skipped, and
   the questions only the author can answer (side effects, taint, version
   history).

## Vocabulary versions

Every loader reads every vocabulary (`1`, `1.0`, `1.1`, `1.2`, `2.0`) in
full; declare the newest, `speclib <name> 2.0`, since the declaration is
what entitles the pack to the newer words (the loader flags each one used
under an older declaration). 2.0 adds `available {PROVIDER SPEC…}` (replaces
`dialects`), pack-level `environment` and `dialect` blocks, and
`refine NAME { … }`. `tcl spec upgrade <pack.tclspec>` rewrites an older
declaration in place (`--check` to report only).

## A 2.0 pack is a Tcl script

A `speclib … 2.0` file is **evaluated**, not walked: it runs in a
deterministic sandbox (no clock, IO, network, processes, environment, or
threads), the vocabulary words are host commands, and what loads is the
snapshot of registrations the run made; `set`, `foreach`, `proc`, and `if`
work between the words. Rules:

- **Canonical form by default** — straight-line registration calls, no
  `proc`, `set`, `foreach`, or computed argument; it is what every generator
  emits and what the studio can edit in place. Reach for a program only when
  repetition is the problem: a shared version is a variable, forty commands
  differing in two fields are a `foreach` over a data table written directly
  above the loop, never assembled across procs.
- **Prefer `-available` rows to branching on `available?`** — a branch
  makes the snapshot one analysis target's answer and marks the pack
  target-dependent; the row states the same fact as data.
- **Read the expansion before shipping** — `tcl spec export FILE` (MCP:
  `spectcl_expand`) renders any snapshot back as canonical source; read it
  as a diff against intent. Expansion is total and contraction is never
  attempted, so the expansion is the whole truth.
- **Scope ambient packages to an environment** — a package the interpreter
  loads before the first byte belongs in an
  `environment NAME { core tcl 8.6; ambient PKG VERSION|keyed KEY; file_extension EXT … }`
  block, not a global claim.
- **Upgrading a 1.x source** — `tcl spec upgrade FILE` rewrites the rows in
  place and preserves every other byte; `--restyle` re-emits the result in
  canonical form (comments and layout dropped). A programmed pack is never
  rewritten.

## Version ranges from release history

One snapshot only says what the library looks like now; stamping
`introduced_version` from it claims every command arrived in the newest
release. With releases available, derive the ranges — the importer writes a
lifecycle field only when two releases disagree and reports the rest as
evidence.

9. **Get the releases.** With network: `tcl spec import --github OWNER/REPO`
   enumerates tags (a leading `v` and a project prefix are stripped:
   `v1.2` → `1.2`, `tcllib-1.20` → `1.20`) and fetches each tarball;
   narrow with `--tag-pattern 'v*'` and `--limit 8`, preview with
   `--list-tags`, set `GITHUB_TOKEN` if rate-limited. From a checkout:
   `git archive` one directory per tag under `snapshots/<ver>/`, then
   `tcl spec import --snapshot 1.0=snapshots/1.0 --snapshot 1.2=…` (a
   snapshot may be a `.zip` or `.tar.gz`). Without a shell: the MCP
   `spec_import` tool takes `snapshots: [{version, path}]` (local only),
   `dialect`, `package`, `complete_history` and returns the same pack,
   ranges, and warnings.
10. **Say whether the history is complete.** `--complete-history`
    (`complete_history: true`) declares the snapshots are every release,
    which alone makes presence in the earliest one an introduction. Claim it
    only after checking the full tag list.
11. **Read the header before the body.** The rendered pack opens with `#`
    lines: releases analysed, every contradiction (a command that vanishes
    and returns, a `package provide` that disagrees with its label),
    `version-gate:` notes for facts no field holds yet, and every field the
    render could not carry. Keep them and resolve each in your report.
12. **Validate as usual** with `mcp__tcl-lsp__spectcl_check`, then merge in
    the hand-written fields the importer cannot infer (taint, side effects,
    better hover prose). The import is a starting point with evidence, not
    an assertion.

## Recognising the common shapes

Cite the evidence line for each; never infer a shape from a name. Corpus
patterns: `docs/design/spec-dsl-examples/external/`.

- **Options** — a `$args` loop matching `-*` (`switch --`, `argparse`,
  `cmdline`); C: `Tcl_GetIndexFromObj` over a `-` table → one `option` row
  per flag; an arm that consumes the next word is a value option, its
  accepted literals become its values.
- **Option values** — closed literal sets in the consuming branch → closed
  values; numeric range checks → an integer domain.
- **Option termination** — explicit `--` handling → declare the `--` option
  (that turns on terminator and taint checks). A scan that always treats
  the last N words as positional → reserved trailing words, not options.
- **N-paired tails** — `foreach {k v} $args`, `llength % 2`, `incr i 2`; C:
  `objc` parity + `i += 2` → stepped arity (plus an exact-count extra where
  one form differs) and a repeat row with stride and excluded trailing
  words.
- **Mutually exclusive options** — joint checks that error, argparse
  `-forbid` → an option-conflict row. argparse `-require` has no field:
  record it as a known limit, never fake it as a conflict.
- **Mode words selecting different tails** — a first-word `switch` to
  unrelated shapes → subcommands with their own arity and roles;
  option-selected modes → note for a maintainer (per-form routing is
  registry-side).
- **Callbacks** — a stored prefix later run with extra words
  (`uplevel #0 [list {*}$cb …]`; C: `Tcl_EvalObjEx` on a built list) → a
  command-prefix position with the appended count you observed (at-least
  when branches differ).
- **Variable writers** — `upvar 1 $name v; set v …` → a variable-write role;
  a caller-chosen level word is frame behaviour — describe it in the notes.
- **Optional trailing arguments** — `llength` ladders with defaults → the
  arity range plus a form synopsis per shape.

## C extensions

Proc inference sees nothing from a compiled extension (`.c`/`.cpp` with
`tcl.h`, `critcl::cproc`/`ccommand`, SWIG `.i`, cffi, a `pkgIndex.tcl` that
`load`s a library). Derive from the C, citing file:line:

- `Tcl_CreateObjCommand(interp, "name", handler, …)` — the name; a
  registration inside another handler is a factory (`defines_command_at`,
  instance commands).
- `Tcl_WrongNumArgs(interp, n, objv, "varName ?value?")` — the synopsis
  verbatim; with the `objc` guards around it, the arity.
- `Tcl_GetIndexFromObj` over a static table — the subcommand or option
  vocabulary (`TCL_EXACT` = strict prefix); the table plus its `switch`
  arms show which flags consume a value.
- `Tcl_ObjSetVar2` / `Tcl_GetVar2Ex` with `objv[i]` → VarWrite / VarRead at
  i; `Tcl_EvalObjEx(objv[i])` → Body and `EVALUATES_CODE`;
  `Tcl_GetChannel` → Channel.
- `Tcl_SetObjResult` with `Tcl_NewIntObj` / `Tcl_NewListObj` → the return
  type; `Tcl_PkgProvide` in `*_Init` → the package and version.
- critcl / cffi / SWIG declarations already state the signature — parse the
  declaration, not the generated C. Shipped `.n` man pages usually carry
  the authoritative synopsis; prefer them for hover text.

Mark every C-derived field with its evidence and list what C cannot tell you
(taint, non-obvious side effects, history) as questions for the author.

$ARGUMENTS
