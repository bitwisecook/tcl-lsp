---
name: spec-author
description: "Build command specs for a private Tcl library: scan its sources with the compiler, infer each command's signature and behaviour, and produce stub sidecars or registry-ready spec drafts. Use when a user's own package commands are unknown to tcl-lsp, when annotating a private library, or when preparing a command-spec contribution."
allowed-tools: mcp__tcl-lsp__read_proc_docs, mcp__tcl-lsp__analyze, mcp__tcl-lsp__command_info, Read, Write, Glob, Grep
---

# Spec Author

Turn a private Tcl library into command specs the tools understand, using
the compiler's own inference — never guesswork from names.

## Background reading (do this first)

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
   target: **private** (stub sidecar, stays in the project) or
   **contribution** (registry `.rs` drafts plus a GitHub issue body).
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
6. Write the output:
   - **private** — one `<dialect>.tcl.stubs` sidecar at the library root,
     stub syntax per the stubs how-to; note anything a stub cannot carry.
   - **contribution** — per-command spec summaries plus an issue body per
     the how-to, and point the user at the Spec Studio to polish and
     render the final `.rs`.
7. Validate: re-run `mcp__tcl-lsp__analyze` on a library file that *uses*
   the commands and confirm the unknown-command diagnostics are gone
   (private) or list what will clear once the specs ship (contribution).
8. Report: commands covered, evidence per guess, anything skipped, and
   open questions only the author can answer (side effects, taint,
   version history).

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
