# Package loading contracts

## Symptom

Completions, hover, or diagnostics are missing for package-gated commands
(stdlib, tcllib, Tk), or cross-file navigation silently drops procs from
resolved packages. iRules procs from other files are not found.

## Operational context

The package loading system sits between the analyser (which detects
`package require` statements) and the LSP features (which filter commands
based on active packages). It spans four layers, all native Rust:

1. **Analyser** (`tcl-compiler`) — extracts `SignaturePackageRequire` from
   source text during the signature scan.
2. **Command registry** (`tcl-registry`) — gates commands via
   `CommandSpec.required_package` / `tcllib_package`.
3. **Package resolver** (`tcl-lsp-core::package_resolver`) — parses
   `pkgIndex.tcl` / `tclIndex` files to find implementation sources and to
   resolve what a `package require` transitively pulls in.
4. **Workspace index** (`tcl-lsp-core::workspace_index`) — aggregates procs,
   classes, invocation sites, `source` references, and `package require`
   declarations across every analysed document.

Per-command package knowledge lives on the `CommandSpec` in the registry's
per-dialect spec packs, never as name-matching in a consumer (see
[AGENTS.md](../../../AGENTS.md), "The registry is the source of truth"):

| Source  | `required_package` value       | Spec pack                          |
|---------|--------------------------------|------------------------------------|
| Stdlib  | e.g. `"http"`, `"msgcat"`      | `tcl-registry/src/commands/stdlib` |
| Tcllib  | derived from `tcllib_package`  | `tcl-registry/src/commands/tcllib` |
| Tk      | `"Tk"`                         | `tcl-registry/src/commands/tk`     |
| iRules  | n/a (no packages on BIG-IP)    | `tcl-registry/src/commands/irules` |

## Decision rules / contracts

### Analyser extraction

1. `package require <name>` and `package require -exact <name> <version>` are
   recorded as `SignaturePackageRequire` in `AnalysisResult.package_requires`.
2. `package provide` is tracked separately (`package_provides`); a file that
   *provides* a package needn't *require* it. `package ifneeded` is not a
   require.
3. Each `SignaturePackageRequire` carries the name, the optional version, its
   source span, and a `conditional` flag set when the require sits inside a
   guarded branch, so version inference does not promote a guarded
   `package require Tcl 8.6` to an unconditional minimum. "Guarded" is
   registry-driven, not a command-name list: the analyser raises
   `conditional_depth` for a body the registry marks
   `Traits::BRANCH_SELECTED_BODY` (`if`, `try`), plus `catch`'s script.
   Per `try` clause, following C Tcl's own semantics (Tcl 9.0.4 `try(n)`;
   `TclNRTryObjCmd` in `generic/tclCmdMZ.c`):

   | `try` clause      | conditional | why |
   |-------------------|-------------|-----|
   | main body         | yes | it always *starts*, but an exception a handler swallows can cut it short, so nothing in it dominates the code after the `try` |
   | `on` / `trap` body | yes | reached only on a matching completion code / `-errorcode` prefix |
   | `finally` body    | **no** | it always runs, whatever the body and handlers did, so whenever control reaches past the `try` it has run |

   The `if` modelling is the same shape: the always-evaluated condition is an
   `ArgRole::Expr` argument and is never depth-bumped, only the
   branch-selected bodies are (issue #1065).
   The background `signature_scan` pre-pass answers the same question more
   coarsely — it walks only `if` / `catch` / `try` bodies and marks *every*
   recursed body conditional, `finally` included. It is a shallow index for
   files never opened in the foreground, not a second authority; the
   analyser's per-clause answer above is the precise one.
4. Version is captured but currently only used by the package resolver and the
   version-gate diagnostics, not by the per-document command filter.

### Registry filtering

5. `CommandSpec.required_package` (or `tcllib_package`, unified by
   `CommandSpec.owning_package()`) names the package a command belongs to;
   `None` means an unconditional core command.
6. Consumers gate a package command on whether its package is active for the
   document — e.g. completion only offers Tk widgets once `Tk` is required
   (`completion.rs`), and hover annotates the owning package (`hover.rs`).
7. `CommandRegistry.provides_package(name)` reports whether any registry spec
   is owned by `name` — the "is this a package the registry knows?" query the
   W120 refinement uses to decide whether a require is resolvable.
8. Tcllib specs set `tcllib_package`; Tk specs set `required_package = "Tk"`
   and do not warn on a missing import, because `wish` auto-loads Tk.

### Package resolver (`pkgIndex.tcl` / `tclIndex`)

9. `PackageResolver::scan_tree(root, max_dirs)` walks a workspace tree (each
   directory and its immediate subdirectories, bounded by `max_dirs`) and
   `scan_path(dir)` scans one directory, mirroring C Tcl's `tclPkgUnknown`.
10. `parse_pkg_index` extracts `package ifneeded <name> <version> <script>`
    entries and the source targets the script would `source` / `load`;
    `parse_tcl_index` extracts the `auto_index` proc → file mappings from a
    `tclIndex`.
11. Index files are tokenised with the **real Tcl lexer**, so nested
    `[file join $dir x]`, quoted, and braced path forms are handled
    structurally — not by regex. The differential tests verify the output
    against a real `tclsh`.
12. `resolve(name, version)` returns the implementation files for the
    best-matching version (exact or prefix match, e.g. `"2"` matches `"2.9"`).
13. `provides(name)` reports whether the scanned database knows `name`.
14. `transitive_available_packages(requires, read_fn)` returns the closure of
    packages the given requires pull in — following each `ifneeded` script's
    own `package require`s — which is how a wrapper package that internally
    `package require Tk` makes `Tk` available (#723).

### Workspace index integration

15. `WorkspaceIndex::add_document(uri, analysis)` records a document's procs,
    classes, invocation sites, `source` targets, and `package require`s;
    `remove_document(uri)` drops every entry from that document before a
    re-index or on close.
16. The server seeds the index from both editor-opened documents (via the
    diagnostics path) and an on-disk scan of the workspace folders
    (`scan_workspace_folders`), so unopened `.tcl` / `.tm` files are covered.
17. Open buffers win over disk-scanned copies: `merge_workspace_scan_results`
    re-checks the live open set at publication time and never overwrites an
    open document's entry with a stale on-disk analysis.

### Missing-`package require` refinement (W120)

18. The analyser's single-file W120 knows only the requires in the current
    document. Two workspace-level refinements are layered on top, both in the
    server's `refine_w120_diagnostics`:
    - **#723 transitive resolution** — a required package is resolved through
      the workspace `pkgIndex.tcl` database; a W120 for a package that the
      requires transitively provide is dropped. If any required package is
      *unknowable* (neither the registry nor the database knows it), it may
      load anything, so every W120 is conservatively dropped.
    - **#804 cross-file inheritance** — see below.

### Cross-file `package require` inheritance (W120, #804)

19. A file need not carry its own `package require` for a command whose package
    was required by an **entry** file that `source`s it.
20. **Automatic (default).** The workspace index's `source` targets and
    `package require`s feed a reverse-reachability walk
    (`tcl-lsp-core::source_graph::ancestor_requires`): a module inherits the
    requires of every file that transitively `source`s it. Only **literal**
    `source path.tcl` targets are followed; a computed `source $dir/x.tcl`
    yields no static edge. Path resolution is
    `source_graph::resolve_source_target` (lexical, no filesystem access); the
    server supplies the URI ↔ path conversion.
21. **Explicit.** `.tcl-lsp.ini [project] entryPoints` lists the entry files
    (relative to the folder root, or absolute). When set, the union of those
    files' requires is treated as available for W120 across the whole folder,
    and the automatic `source`-graph inheritance is **disabled** for that
    folder.
22. The inherited requires are merged with the document's own before the #723
    transitive resolution runs, so an inherited `package require myWrapper`
    still (transitively) pulls in whatever `myWrapper` provides.

### iRules cross-file equivalent

23. iRules do not support `package require` on BIG-IP, so W120 never applies
    to the `f5-irules` dialect (the refinement early-returns when the registry
    has no `package` command).
24. iRules procs are instead globally visible across files through the same
    workspace index proc aggregation the LSP cross-document features use.

### Split packages

25. A single `package ifneeded` script may `source` multiple files
    (`source [file join $dir a.tcl]; source [file join $dir b.tcl]`); each is
    extracted independently by the lexer-driven parser, so no regex
    semicolon-capture limitation applies.
26. A single `pkgIndex.tcl` may declare multiple unrelated packages — each
    `package ifneeded` line is parsed independently.

## File-path anchors

- `tcl-compiler/src/signature_scan/types.rs` — `SignaturePackageRequire`,
  `SignatureSource`.
- `tcl-compiler/src/analyser/handlers.rs` — `handle_source_command`, package
  require extraction.
- `tcl-compiler/src/analyser/diagnostics/unresolved.rs` —
  `emit_missing_package_require_diagnostics` (single-file W120).
- `tcl-registry/src/spec.rs` — `CommandSpec.required_package`,
  `tcllib_package`, `owning_package()`.
- `tcl-registry/src/registry.rs` — `provides_package()`.
- `tcl-registry/src/commands/{stdlib,tcllib,tk,irules}` — dialect spec packs.
- `tcl-lsp-core/src/package_resolver.rs` — `PackageResolver`,
  `parse_pkg_index`, `parse_tcl_index`, `transitive_available_packages`.
- `tcl-lsp-core/src/workspace_index.rs` — `WorkspaceIndex`,
  `WorkspacePackageRequire`, `WorkspaceSource`,
  `source_ancestor_package_requires`.
- `tcl-lsp-core/src/source_graph.rs` — `resolve_source_target`,
  `resolve_under`, `ancestor_requires`.
- `tcl-lsp-server/src/lib.rs` — `refine_w120_diagnostics`,
  `refine_workspace_w120`, `compute_inherited_requires`,
  `w120_inheritance_config`, `scan_workspace_folders`,
  `build_package_resolver`.
- `tcl-lsp-server/src/config_ini.rs` — `settings_from_ini` (`entryPoints`,
  `libraryPaths`).

## Failure modes

- **Missing completions**: a `package require` not detected → the package's
  gated commands stay hidden.
- **False W120 diagnostics**: a require not recorded, or the workspace not yet
  scanned, → W120 fires for a command that is actually imported (directly, via
  a wrapper package, or via an entry file that sources the module).
- **Stale package database**: the resolver not rebuilt after a library-path or
  workspace change → old `provides` / `resolve` results. `scan_workspace_folders`
  rebuilds it.
- **Unresolved cross-file inheritance**: a computed `source $dir/x.tcl` yields
  no `source`-graph edge, so the sourced module inherits nothing — configure
  `entryPoints` for that case.
- **iRules proc not found**: a file not indexed → its procs invisible to other
  iRules files.

## Test anchors

- `tcl-lsp-core/src/package_resolver.rs` (tests) — pkgIndex / tclIndex parsing
  and resolution, differential-tested against `tclsh`.
- `tcl-lsp-core/src/workspace_index.rs` (tests) — index aggregation, the
  `source`-graph walk, and `package require` recording.
- `tcl-lsp-core/src/source_graph.rs` (tests) — path resolution and ancestor
  reachability.
- `tcl-lsp-server/src/lib.rs` (tests) — `refine_w120_*`,
  `compute_inherited_requires`, and the push/pull integration
  (`source_graph_inheritance_suppresses_w120_in_sourced_module`,
  `explicit_entry_point_config_suppresses_w120_project_wide`,
  `pull_path_applies_w120_package_refinement`).
- `tcl-registry/tests/registry_sweep.rs` — registry-wide command / package
  coverage.

## Cross-reference: tclpkg

For project-local package management (manifests, lockfiles, CAS, virtual
environments), see [tclpkg architecture](../tclpkg-architecture.md) and
the [how-to guide](../../kcs/kcs-howto-manage-tcl-packages.md).

## Discoverability

- [Design doc index](../README.md)
- [Workspace indexing contracts](workspace-indexing.md)
- [Command registry and event model](command-registry-event-model.md)
- [LSP feature providers](lsp-feature-providers.md)
- [LSP diagnostics publication](lsp-diagnostics-publication.md)
- [tclpkg architecture](../tclpkg-architecture.md)
