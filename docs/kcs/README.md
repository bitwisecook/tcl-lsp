# KCS index

This folder holds Knowledge-Centered Service (KCS) notes. A KCS note is a
small, searchable answer to one question, written in plain English for a
named audience.

Every KCS note belongs to one of six categories:

- **Issue** — "Why is X not working, and how do I fix it?"
- **Q&A** — "What is X?" / "When should I use Y?"
- **How-To** — "How do I do X?"
- **Functionality** — "What does command/feature/tool X do, and how do I
  use it?"
- **Diagnostic** — per-code page for an E/W/S/T/IRULE diagnostic.
- **Optimisation** — per-code page for an O-code optimiser rewrite.

If your content does not fit one of these six categories, you are writing
a design doc. Put it under [`../design/`](../design/README.md) instead.

## How to write a KCS note

1. Pick a category and copy the matching template from
   [`templates/`](templates/README.md).
2. Follow the style guide in [`STYLE.md`](STYLE.md). The short-form rules
   are also listed in [`AGENTS.md`](../../AGENTS.md) under "Knowledge base
   and documentation".
3. Link the new note from the appropriate section below.
4. Add a cross-link to [`docs/GLOSSARY.md`](../GLOSSARY.md) for any
   complex term you use.

## Issues

- [kcs-issue-lsp-features-are-missing.md](kcs-issue-lsp-features-are-missing.md)
  — squiggles, hovers, and completions do not appear in VS Code and
  you want to know whether the Tcl Language Server started at all.
- [kcs-issue-stale-compiler-cache.md](kcs-issue-stale-compiler-cache.md)
  — stale incremental cache produces wrong diagnostics.
- [kcs-issue-range-drift.md](kcs-issue-range-drift.md) — diagnostic or
  hover ranges point at the wrong span.
- [kcs-issue-duplicate-diagnostics.md](kcs-issue-duplicate-diagnostics.md)
  — the same finding is reported twice.
- [kcs-issue-wasm-arithmetic-divergence.md](kcs-issue-wasm-arithmetic-divergence.md)
  — `expr {1 / 0}` returns 0 in compiled WASM instead of raising
  `divide by zero`.
- [kcs-issue-wasm-ignores-missing-var.md](kcs-issue-wasm-ignores-missing-var.md)
  — `expr {$undefined + 1}` returned `1` in compiled WASM instead of
  raising `can't read "undefined": no such variable` (issue #263).
- [kcs-issue-counter-numtests-array-init.md](kcs-issue-counter-numtests-array-init.md)
  — tcltest's `numTests(Failed)` reads as an empty string in the
  compiled counter-bundle run.

## Q&A

- [kcs-qa-when-to-restart-server.md](kcs-qa-when-to-restart-server.md) —
  when (and when not) to restart the Tcl Language Server.

## How-Tos

- [kcs-howto-add-compiler-pass.md](kcs-howto-add-compiler-pass.md) — add
  a new pass to the compiler pipeline.
- [kcs-howto-ir-cfg-ssa-diagnostics.md](kcs-howto-ir-cfg-ssa-diagnostics.md)
  — debug an IR, CFG, or SSA diagnostic end-to-end.
- [kcs-howto-work-on-fuzz-findings.md](kcs-howto-work-on-fuzz-findings.md)
  — triage, fix, test, and close a differential-fuzzer finding.
- [kcs-howto-author-tcl-test-scripts.md](kcs-howto-author-tcl-test-scripts.md)
  — write small Tcl scripts for parser, analysis, and bytecode tests.
- [kcs-howto-author-irule-test-scripts.md](kcs-howto-author-irule-test-scripts.md)
  — write iRule scripts for event-flow and diagnostic tests.
- [kcs-howto-author-screenshot-samples.md](kcs-howto-author-screenshot-samples.md)
  — write sample files and cursor marker comments for screenshots.
- [kcs-howto-manage-tcl-packages.md](kcs-howto-manage-tcl-packages.md)
  — add, install, and lock Tcl package dependencies with tclpkg.
- [kcs-how-to-run-tcltest-bundles.md](kcs-how-to-run-tcltest-bundles.md)
  — run the Tcl 9 tcltest test files through the WASM runtime and
  interpret the triage roll-up.
- [kcs-howto-suppress-diagnostics.md](kcs-howto-suppress-diagnostics.md)
  — turn a diagnostic, warning, optimisation, or shimmer off inline,
  file-wide, per-project, per-editor, or globally.

## Tcl 9 correctness

- [kcs-tcl9-test-corpus.md](kcs-tcl9-test-corpus.md) — inventory of the
  upstream Tcl 9.0.3 test corpus grouped by subsystem, in-scope vs
  deferred-by-design.
- [kcs-tcl9-triage.md](kcs-tcl9-triage.md) — per-test-file triage table
  fed by the harness JSON report.

## Functionality (commands, features, and tools)

65 per-feature KCS notes live under [`features/`](features/README.md).
The `help` subcommand, the MCP `help` tool, and the VS Code `/help`
chat command all read these files at runtime to build their feature
catalogues.

## Diagnostics and optimisations (per-code pages)

120 per-code KCS notes live under [`codes/`](codes/README.md) — 92
diagnostic pages (E, W, S, T, and IRULE families) and 28 optimisation
pages (O family). Each page follows the diagnostic or optimisation
template, tags the compiler pass that produces it, explains in plain
English why the check exists, shows a triggering example and the fix,
and links to the [glossary](../GLOSSARY.md) and the relevant
[compiler design doc](../design/compiler/README.md).

## Templates

- [templates/README.md](templates/README.md) — index of the six KCS
  templates.

## Style guide

- [STYLE.md](STYLE.md) — the full KCS style guide with worked examples.

## Where technical documentation lives

Design docs, contracts, interfaces, data-structure references, and
architecture narratives live under [`../design/`](../design/README.md).
The [glossary](../GLOSSARY.md) is the single source of truth for complex
terms.
