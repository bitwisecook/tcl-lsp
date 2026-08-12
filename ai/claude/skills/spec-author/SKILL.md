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

$ARGUMENTS
