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
- [kcs-issue-parallel-worktree-builds-serve-stale-artefacts.md](kcs-issue-parallel-worktree-builds-serve-stale-artefacts.md)
  — builds in one git worktree fail or pass with artefacts from a
  sibling checkout because the worktrees share one cargo target
  directory.
- [kcs-issue-memory-grows-while-editing.md](kcs-issue-memory-grows-while-editing.md)
  — the language server's memory use climbs with every keystroke and never
  comes back down.
- [kcs-issue-problems-not-retained-after-closing-files.md](kcs-issue-problems-not-retained-after-closing-files.md)
  — a file's problems and File Explorer badge vanish after its editor
  tab is closed.
- [kcs-issue-range-drift.md](kcs-issue-range-drift.md) — diagnostic or
  hover ranges point at the wrong span.
- [kcs-issue-highlight-drops-closing-delimiter.md](kcs-issue-highlight-drops-closing-delimiter.md)
  — a highlight over a braced word covers `{$condition` instead of
  `{$condition}`, dropping the closing delimiter.
- [kcs-issue-array-element-not-highlighted-as-variable.md](kcs-issue-array-element-not-highlighted-as-variable.md)
  — a by-reference variable-name argument (`set arr(key) 1`,
  `info exists arr(key)`) is not highlighted as a variable.
- [kcs-issue-subcommand-script-body-not-highlighted.md](kcs-issue-subcommand-script-body-not-highlighted.md)
  — a subcommand's script argument (`console eval { ... }`) stays one opaque
  string instead of recursing into keyword/variable/comment highlighting.
- [kcs-issue-apply-lambda-body-not-highlighted-via-list-quoting.md](kcs-issue-apply-lambda-body-not-highlighted-via-list-quoting.md)
  — commands inside an `apply {argList body}` lambda stay one opaque string
  when `apply` is reached indirectly through `[list apply {...} $x]`
  (the pkgIndex.tcl `package ifneeded ... [list apply {dir {...}} $dir]`
  idiom), even though a direct `apply {...}` call highlights fine.
- [kcs-issue-classes-made-by-a-class-factory-are-invisible.md](kcs-issue-classes-made-by-a-class-factory-are-invisible.md)
  — a class made by a user-defined `TclOO` metaclass, a member whose
  signature arrives through `{*}` expansion, a class named by a `foreach`
  loop variable, or a command head built from a namespace variable is
  missing from the outline and resolves nowhere.
- [kcs-issue-w129-safe-interp-hidden-command-via-bracket-indirection.md](kcs-issue-w129-safe-interp-hidden-command-via-bracket-indirection.md)
  — W129 (a command hidden in a safe interpreter) does not warn when the
  hidden command is reached only through a `[...]` bracket substitution —
  most importantly the `package ifneeded ... [list apply {...} $dir]`
  deferred-command idiom.
- [kcs-issue-w129-false-positive-on-control-transfer-commands.md](kcs-issue-w129-false-positive-on-control-transfer-commands.md)
  — W129 wrongly fires on `break`, `continue`, `yield`, `yieldto`, and
  `tailcall` in a safe-interpreter body, and the *Inline proc* code action
  is missing on `file`, `exec`, `open`, and seven other commands — two
  symptoms of one trait-flag bit collision.
- [kcs-issue-false-diagnostics-inside-a-multi-word-eval.md](kcs-issue-false-diagnostics-inside-a-multi-word-eval.md)
  — a multi-word `eval`, `uplevel`, or `namespace eval` draws a false E002
  "wrong number of arguments", and a variable the call sets is still
  reported as read before it is set (W210).
- [kcs-issue-always-true-condition-in-a-sourced-library-file.md](kcs-issue-always-true-condition-in-a-sourced-library-file.md)
  — I230 says a condition is always true in a library file whose procedure
  is really called with different values from another file.
- [kcs-issue-irule-word-operator-is-not-analysed.md](kcs-issue-irule-word-operator-is-not-analysed.md)
  — an iRules word operator (`contains`, `starts_with`, …) is neither
  folded by `tcl opt` nor reported by the analyser, because the file's
  dialect never reached the optimiser or the expression parser.
- [kcs-issue-duplicate-diagnostics.md](kcs-issue-duplicate-diagnostics.md)
  — the same finding is reported twice.
- [kcs-issue-counter-numtests-array-init.md](kcs-issue-counter-numtests-array-init.md)
  — tcltest's `numTests(Failed)` reads as an empty string in the
  compiled counter-bundle run.
- [kcs-issue-reconstruct-a-stress-test-failure.md](kcs-issue-reconstruct-a-stress-test-failure.md)
  — a stress-test suite run failed and you want to reconstruct it from
  the `STRESS_FAILURE:` reproduction bundle.

## Q&A

- [kcs-qa-which-commands-are-available-in-a-dialect.md](kcs-qa-which-commands-are-available-in-a-dialect.md)
  — how the dialect profile decides command availability (embedded Tcl
  base + the iRules disable list).
- [kcs-qa-when-to-restart-server.md](kcs-qa-when-to-restart-server.md) —
  when (and when not) to restart the Tcl Language Server.
- [kcs-qa-query-vs-grep-vs-rename.md](kcs-qa-query-vs-grep-vs-rename.md) —
  which `f5` verb to pick for filter / find / rename tasks.
- [kcs-qa-tcl-lsp-annotations.md](kcs-qa-tcl-lsp-annotations.md) — which
  `# tcl-lsp:` and `# noqa` comments the analyser understands.
- [kcs-qa-how-tcl-lsp-loads-configuration.md](kcs-qa-how-tcl-lsp-loads-configuration.md)
  — the five places the server reads configuration from, and which
  layer wins when they disagree.
- [kcs-qa-what-config-sections-are-valid.md](kcs-qa-what-config-sections-are-valid.md)
  — the nine INI sections (seven shared plus the location-specific
  `[global]` and `[project]`), their keys, and which values are valid.
- [kcs-qa-rust-shim-env-vars.md](kcs-qa-rust-shim-env-vars.md) — what the
  `TCL_LSP_RUST_*` environment variables do and when to set them as the
  Python-to-Rust rewrite lands in chunks.
- [kcs-qa-how-tcl-parses-lists.md](kcs-qa-how-tcl-parses-lists.md) — how
  Tcl splits a list (and a `proc` / method parameter list) into elements
  on whitespace, and how braces, quotes, and a trailing backslash line
  continuation move those boundaries.
- [kcs-qa-how-are-command-names-resolved.md](kcs-qa-how-are-command-names-resolved.md)
  — which definition a bare, relative, or absolute command name
  dispatches to, and the one shared algorithm every backend conforms to.
- [kcs-qa-where-can-i-call-my-next-self-and-link.md](kcs-qa-where-can-i-call-my-next-self-and-link.md)
  — why `link`, `my`, `next`, `nextto`, `self`, and `classvariable` are
  unknown commands outside a `TclOO` method body, which bodies count
  (including why an `apply` lambda does not), and how the fully qualified
  `::oo::Helpers::…` spellings differ.
- [kcs-qa-tcltest-package-support.md](kcs-qa-tcltest-package-support.md) —
  how the server models the `tcltest` package, its `test` / `configure`
  options, and their per-version availability across Tcl 8.4-9.0.
- [kcs-qa-why-diagnostics-appear-progressively.md](kcs-qa-why-diagnostics-appear-progressively.md)
  — why a large file's diagnostics arrive in two waves (a fast tier of
  single-file checks, then the complete deep tier), and why W120/W123 are
  held back from the first wave.
- [kcs-qa-tclpkg-manifest-diagnostics.md](kcs-qa-tclpkg-manifest-diagnostics.md)
  — what the editor checks inside a `tclpkg.tcl` package manifest, and
  why its directives no longer draw "Unknown command" warnings.
- [kcs-qa-why-w112-w118-have-no-quick-fix.md](kcs-qa-why-w112-w118-have-no-quick-fix.md)
  — why the trailing-whitespace and line-ending hints stay quick-fix-free:
  the document formatter is the safe, already-existing fix.
- [kcs-qa-when-is-a-proc-parameter-treated-as-a-constant.md](kcs-qa-when-is-a-proc-parameter-treated-as-a-constant.md)
  — when the analyser binds a procedure parameter to a compile-time
  literal from its call sites, which indirect calls (`$cmd args`, callback
  prefixes, `eval`) count as call sites too, and why adding one of them
  makes the folded diagnostics disappear.

## How-Tos

- [kcs-howto-lock-down-tcl-pkg.md](kcs-howto-lock-down-tcl-pkg.md) — deploy a
  locked-down `tcl pkg` policy for an organisation: a sandbox floor developers
  cannot loosen, registry allow-lists, operator scanning hooks, and gating
  package build scripts.
- [kcs-howto-build-multiplatform-vsix.md](kcs-howto-build-multiplatform-vsix.md)
  — build the universal VS Code `.vsix` that bundles a native server per
  platform, and add a new platform.
- [kcs-howto-add-compiler-pass.md](kcs-howto-add-compiler-pass.md) — add
  a new pass to the compiler pipeline.
- [kcs-howto-ir-cfg-ssa-diagnostics.md](kcs-howto-ir-cfg-ssa-diagnostics.md)
  — debug an IR, CFG, or SSA diagnostic end-to-end.
- [kcs-howto-array-element-ssa-typing.md](kcs-howto-array-element-ssa-typing.md)
  — the per-element array SSA contract: may-def joins, synthetic-def
  skips, base-keyed policy checks.
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
- [kcs-howto-run-the-stress-test-suites.md](kcs-howto-run-the-stress-test-suites.md)
  — run the issue #829 robustness stress suites (direct-infrastructure
  and LSP-API) and reconstruct a failure from its reproduction bundle.
- [kcs-howto-suppress-diagnostics.md](kcs-howto-suppress-diagnostics.md)
  — turn a diagnostic, warning, optimisation, or shimmer off inline,
  file-wide, per-project, per-editor, or globally.
- [kcs-howto-hide-diagnostics-in-diff-views.md](kcs-howto-hide-diagnostics-in-diff-views.md)
  — hide Tcl diagnostics in VS Code diff and compare editors with
  `tclLsp.suppressDiagnosticsInDiffEditors`.
- [kcs-howto-configure-project-entry-points.md](kcs-howto-configure-project-entry-points.md)
  — stop W120 warnings in files loaded by an entry file, via the
  automatic `source` graph or a manual `entryPoints` list.
- [kcs-howto-bind-sublime-tcl-commands.md](kcs-howto-bind-sublime-tcl-commands.md)
  — add keyboard shortcuts for the Tcl package's commands in Sublime
  Text using the bundled example keymap.
- [kcs-howto-annotate-commands-with-stubs.md](kcs-howto-annotate-commands-with-stubs.md)
  — declare third-party Tcl commands (sqlite `eval`, vendor builtins,
  factory-created instance commands) so the call graph, arity checker,
  and trait inferencer understand them.
- [kcs-howto-add-command-registry-package.md](kcs-howto-add-command-registry-package.md)
  — add first-class registry support for a Tcl package (sqlite3,
  tcllib, etc.) so the shipped distribution recognises its commands
  with hover docs, completion, arity checks, and side-effect
  classification.
- [kcs-howto-readdress-virtuals-with-query.md](kcs-howto-readdress-virtuals-with-query.md)
  — bulk-readdress virtual servers into a new subnet with `f5 query`.
- [kcs-howto-migrate-partition-with-query.md](kcs-howto-migrate-partition-with-query.md)
  — move every object from one partition into another, including
  route-domain transforms.
- [kcs-howto-compose-query-streams.md](kcs-howto-compose-query-streams.md)
  — filter and transform streams of BIG-IP objects with `select`,
  `map`, `any`, `all`, `sort`, `unique`, and friends.
- [kcs-howto-audit-config-with-query.md](kcs-howto-audit-config-with-query.md)
  — audit a config for orphans, naming-convention violations, port
  policy, partition leaks, and pool-member sanity using `f5 query`.
- [kcs-howto-audit-server-certs-with-query.md](kcs-howto-audit-server-certs-with-query.md)
  — verify the cert on each device's `sys file ssl-cert` matches
  the cert each virtual is actually serving; find devices where a
  cert push failed in a multi-tier deployment.
- [kcs-howto-reproduce-http-monitor-with-query.md](kcs-howto-reproduce-http-monitor-with-query.md)
  — reproduce an `ltm monitor http(s)` from your laptop, honouring
  the 5,120-byte response-check ceiling (F5 KB K3451) so the
  result matches what the device sees.
- [kcs-howto-verify-migration-before-after-with-query.md](kcs-howto-verify-migration-before-after-with-query.md)
  — verify a migration before/after straight from two UCS files:
  config parity (IPs, self-IP lockdowns, monitors, certs) with a
  match column, plus live probes that prove the VIPs still listen,
  serve the same cert, and answer `GET /` the same way.
- [kcs-howto-read-encrypted-ucs-archives.md](kcs-howto-read-encrypted-ucs-archives.md)
  — run any `f5` verb against a passphrase-protected UCS (`tmsh save
  sys ucs ... passphrase`); supply the passphrase via
  `F5_UCS_PASSPHRASE` or a secure prompt (`extract` / `convert` add
  `--passphrase-env` / `--no-passphrase-prompt`).
- [kcs-howto-cross-config-transforms-with-query.md](kcs-howto-cross-config-transforms-with-query.md)
  — compose multi-step transformations (rename + readdress + policy
  edit) across the config in one `;`-separated query.
- [kcs-howto-rewrite-pool-refs-in-irules.md](kcs-howto-rewrite-pool-refs-in-irules.md)
  — rename a pool everywhere, including inside iRule bodies.
- [kcs-howto-find-objects-by-query.md](kcs-howto-find-objects-by-query.md)
  — filter BIG-IP objects by arbitrary property predicates.
- [kcs-howto-script-against-f5-query-from-python.md](kcs-howto-script-against-f5-query-from-python.md)
  — drive the query engine from a Python script via the `f5q`
  alias, get typed `ObjectRef` / `PathRef` results back, render
  with a built-in plugin, or ship your own renderer in one
  `@renderer` decorator.
- [kcs-tcl-corner-cases.md](kcs-tcl-corner-cases.md)
  — empirical reference of Tcl 9.0.3 variable-handling behaviour
  with a machine-runnable probe set in `tests/data/tcl_probes_full.tcl`.

## Tcl 9 correctness

- [kcs-tcl9-test-corpus.md](kcs-tcl9-test-corpus.md) — inventory of the
  upstream Tcl 9.0.4 test corpus grouped by subsystem, in-scope vs
  deferred-by-design.
- [kcs-tcl9-triage.md](kcs-tcl9-triage.md) — per-test-file triage table
  fed by the harness JSON report.

## Functionality (commands, features, and tools)

81 per-feature KCS notes live under [`features/`](features/README.md).
The `help` subcommand, the MCP `help` tool, and the VS Code `/help`
chat command all read these files at runtime to build their feature
catalogues.

## Diagnostics and optimisations (per-code pages)

156 per-code KCS notes live under [`codes/`](codes/README.md) — 125
diagnostic pages (E, W, S, T, and IRULE families) and 31 optimisation
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
