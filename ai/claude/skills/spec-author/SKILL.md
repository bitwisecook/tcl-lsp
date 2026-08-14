---
name: spec-author
description: "Build command specs for a private Tcl library: scan its sources with the compiler, infer each command's signature and behaviour, and produce stub sidecars or registry-ready spec drafts. Use when a user's own package commands are unknown to tcl-lsp, when annotating a private library, or when preparing a command-spec contribution."
allowed-tools: mcp__tcl-lsp__read_proc_docs, mcp__tcl-lsp__analyze, mcp__tcl-lsp__command_info, mcp__tcl-lsp__spectcl_check, mcp__tcl-lsp__spec_import, Read, Write, Glob, Grep, Bash
---

# Spec Author

Turn a private Tcl library into command specs the tools understand, using
the compiler's own inference — never guesswork from names.

## Background reading (do this first)

- `docs/design/spec-dsl-examples/README.md` — **the frozen SpecTcl
  syntax specification.** SpecTcl is the output format for everything
  this skill produces; read the whole memo, and study the eleven
  `*.tclspec` ports beside it as worked examples before writing a line.
- `docs/kcs/kcs-howto-create-command-specs-without-rust.md` — the workflow
  and where each artefact goes.
- `docs/design/compiler/command-registry.md` — what every spec field means.
- `docs/design/contracts/proc-arg-traits.md` — how inferred parameter
  traits map to argument roles and trait bits.
- `docs/kcs/kcs-howto-annotate-commands-with-stubs.md` — stub syntax and
  its limits (no subcommands, no arity checking, sidecar named
  `<dialect>.tcl.stubs`).

## Steps

1. Ask (or infer from `$ARGUMENTS`) the library's source directory and the
   target: **private** (a SpecTcl pack, stays in the project) or
   **contribution** (the same SpecTcl pack plus a GitHub issue body).
2. Glob the library's `.tcl` files. Note `package provide` and any
   `namespace export` — they set `required_package` gating and the
   public-command surface.
3. For each file, call `mcp__tcl-lsp__read_proc_docs` with its contents.
   This returns every proc with its params, parsed docstring, and
   **inferred param traits** — the same inference the Spec Studio uses.
4. Derive each command's spec from evidence, citing it:
   - arity from the parameter list (defaults optional, trailing `args`
     variadic);
   - argument roles and trait bits from the param traits, mapped per the
     proc-arg-traits contract;
   - hover text from the docstring;
   - `required_package` from `package provide`.
   Only claim what the evidence shows. A parameter with no traits is a
   plain `Value`.
5. Check each name with `mcp__tcl-lsp__command_info` — a clash with a
   built-in needs the user's attention, not a silent overwrite.
6. Write the output — **SpecTcl in both cases**:
   - **private** — one `<library>.tclspec` pack at the library root,
     written strictly to the frozen syntax (schema keys as property
     words, catalogue spellings verbatim, hook bodies only where the
     evidence demands one and always with their family's calling
     convention). Until the pack loader ships, also emit the stub
     sidecar as the working fallback and say so.
   - **contribution** — the same `.tclspec` pack plus an issue body per
     the how-to; the Spec Studio renders the final `.rs` from it.
   Validate the pack with `mcp__tcl-lsp__spectcl_check` — pass the pack
   source and the target `dialect`. It loads the pack through the real
   loader and reports, per command, the draft fields the declaration
   actually set; every loader **notice** (`line`, `context`, `reason`) —
   each one a word that was dropped, so a typo'd trait or an unknown
   property shows up here and nowhere else; every declared **hook** with
   its family and whether it is shape-cacheable; and every
   **collision** with a shipped command name for that dialect. Fix every
   notice, and every `shipped-spec-wins` collision (rename, or add
   `-override` if replacing the shipped spec is genuinely intended — see
   step 5). An uncacheable hook is legal, not a finding: report the cost
   rather than removing the hook. Only if the tool is unavailable,
   self-check each command against the syntax memo's coverage matrix and
   say validation was manual.
7. Validate: re-run `mcp__tcl-lsp__analyze` on a library file that *uses*
   the commands and confirm the unknown-command diagnostics are gone
   (private) or list what will clear once the specs ship (contribution).
8. Report: commands covered, evidence per guess, anything skipped, and
   open questions only the author can answer (side effects, taint,
   version history).

## Deriving version ranges from release history

Steps 1–8 read *one* snapshot of the library, so they can only say what it
looks like now. Stamping `introduced_version` from that snapshot's own
`package provide` claims every command arrived in the newest release, which
is almost never true. If the library has releases, derive the ranges from
them instead — the importer only writes a lifecycle field when two releases
disagree about whether something exists, and reports everything else as
evidence.

9. **Get the releases.** In preference order:
   - **Network available** — `tcl spec import --github OWNER/REPO` enumerates
     the repository's tags, maps each to a version label (a leading `v` and a
     project prefix are stripped: `v1.2` → `1.2`, `tcllib-1.20` → `1.20`) and
     fetches each release tarball. Narrow the set first:
     `--tag-pattern 'v*'` keeps only matching tags (`*` any run, `?` one
     character, the whole tag must match) and `--limit 8` keeps the newest
     eight. Run it once with `--list-tags` to see exactly what would be
     fetched before fetching anything. Set `GITHUB_TOKEN` if the
     unauthenticated rate limit bites; standard proxy variables are honoured.
   - **No network, but a checkout** — export one directory per release and
     label it yourself:

     ```sh
     git clone --bare https://…/pkg.git pkg.git
     git -C pkg.git tag --list 'v*'
     for tag in v1.0 v1.2 v2.0; do
         mkdir -p snapshots/${tag#v}
         git -C pkg.git archive "$tag" | tar -x -C "snapshots/${tag#v}"
     done
     ```

     then `tcl spec import --snapshot 1.0=snapshots/1.0 --snapshot
     1.2=snapshots/1.2 --snapshot 2.0=snapshots/2.0`. A snapshot path may
     equally be a `.zip` or `.tar.gz` — release archives need no unpacking.
   - **Already-local artefacts, no shell** — the MCP `spec_import` tool takes
     `snapshots: [{version, path}]` (local directories or archives only; it
     has no fetcher by design), plus `dialect`, `package` and
     `complete_history`, and returns the same pack, per-command ranges and
     warnings as JSON.
10. **Say whether the history is complete.** `--complete-history` (MCP:
    `complete_history: true`) declares the snapshots are *every* release, which
    is the only thing that makes presence in the earliest one an introduction.
    It is off by default. Claim it only when you have checked the full tag
    list — a wrong `introduced_version` cannot be told from a derived one
    afterwards.
11. **Read the header before the body.** The rendered pack opens with `#`
    comment lines carrying the releases analysed, every contradiction the
    derivation raised (a command that vanishes and returns, a snapshot whose
    `package provide` disagrees with its label), the `version-gate:` notes for
    facts no spec field can hold yet (per-argument value ranges), and every
    field the render could not carry. Keep them: they are the evidence for
    each range, and a reviewer overruling a guess needs them. Resolve or
    answer each one in your report rather than deleting it.
12. **Validate as usual.** Feed the rendered pack to
    `mcp__tcl-lsp__spectcl_check` exactly as step 6 does, then merge in the
    hand-written fields the importer cannot infer (taint, side effects,
    hover prose you have better wording for). The import is a starting point
    carrying its evidence, not an assertion.

## Recognising the common shapes

The corpus-wide patterns (see
`docs/design/spec-dsl-examples/external/`), with the evidence that
identifies each and the spec fields it extracts to. Always cite the
evidence line; never infer a shape from the command's name.

- **Options.** Tcl: a loop over `$args` matching `-*` (`switch --`,
  `argparse`, `cmdline`). C: `Tcl_GetIndexFromObj` over a `-`-string
  table with `switch` arms. → one `option` row per flag; an arm that
  consumes the next word makes it a value option; the arm's accepted
  literals become its values.
- **Option values.** Closed literal sets in the consuming branch
  (`switch`/`lsearch` tables, validation `if`s) → closed values;
  numeric range checks → an integer domain.
- **Option termination.** Explicit `--` handling (`$arg eq "--"` /
  `strcmp("--")`) → declare the `--` option (this is what turns on the
  terminator and taint checks). A scan that always treats the last N
  words as positional → reserved trailing words, not more options.
- **N-paired tails.** `foreach {k v} $args`, parity checks
  (`llength % 2`), `incr i 2` loops; C: `objc` parity + `i += 2`. →
  stepped arity (with an exact-count extra where one form differs) and
  a repeat row with the stride and any excluded trailing words.
- **Mutually exclusive options.** Joint checks that error
  (`$a && $b → error`, argparse `-forbid`) → an option-conflict row.
  argparse `-require` (one option demanding another) has no spec field
  today — record it in the report as a known limit, never fake it with
  a conflict.
- **Mode words selecting different tails.** A first-word `switch` to
  unrelated shapes → subcommands (each with its own arity and roles);
  when the modes are option-selected instead, note it for a maintainer
  — per-form routing is a registry-side feature.
- **Callbacks.** A stored command prefix later run with extra words
  (`uplevel #0 [list {*}$cb …]`; C: `Tcl_EvalObjEx` on a built list) →
  a command-prefix position with the appended count you observed —
  count the words actually appended, and use at-least when branches
  differ.
- **Variable writers.** `upvar 1 $name v; set v …` → a variable-write
  role at that argument; a caller-chosen level word means frame
  behaviour — describe it in the notes rather than guessing fields.
- **Optional trailing arguments.** `llength` ladders with defaults →
  the arity range, plus a form synopsis per meaningful shape.

## C extensions (commands created through the C ABI)

Proc inference sees nothing when commands come from a compiled
extension. Detect that case (`.c`/`.cpp` with `#include <tcl.h>`,
`critcl::cproc`/`ccommand` blocks, SWIG `.i`, cffi declarations, a
`pkgIndex.tcl` that `load`s a shared library) and derive the spec from
the C source instead, citing file:line for every claim:

- `Tcl_CreateObjCommand(interp, "name", handler, …)` — the command name
  and where its evidence lives. Registrations made *inside* another
  handler are factories: `defines_command_at` / instance commands.
- `Tcl_WrongNumArgs(interp, n, objv, "varName ?value?")` — the richest
  single artefact: it is the synopsis verbatim, and with the `objc`
  guards around it (`objc < 3`, `objc != 4`) it fixes the arity.
- `Tcl_GetIndexFromObj` over a static string table — the subcommand or
  option vocabulary; `TCL_EXACT` means strict prefix matching. Option
  tables plus their `switch` arms show which flags consume a value.
- `Tcl_ObjSetVar2`/`Tcl_GetVar2Ex` with an `objv[i]` name → VarWrite /
  VarRead role at i; `Tcl_EvalObjEx(objv[i])` → Body role and
  `EVALUATES_CODE`; `Tcl_GetChannel` → Channel role.
- `Tcl_SetObjResult` with `Tcl_NewIntObj`/`Tcl_NewListObj`/… → the
  return type; `Tcl_PkgProvide` in the `*_Init` function → the required
  package and its version.
- critcl/cffi/SWIG declarations already state the signature — parse the
  declaration, not the generated C.
- Shipped `.n`/man pages often carry the authoritative synopsis and
  option docs; prefer them for hover text over inferring prose.

Mark every C-derived field with its evidence the same way as inferred
ones, and list what C cannot tell you (taint, side effects beyond the
obvious, version history) as questions for the author.

$ARGUMENTS
