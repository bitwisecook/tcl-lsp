# Package loading contracts

How a `package require` becomes a set of visible commands and reachable procs.
This layer decides whether package-gated commands (stdlib, tcllib, Tk) reach
completions, hover, and diagnostics, and whether cross-file navigation finds
procs from a resolved package or from another iRules file, so read it before
changing what a `package require` pulls in.

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
3. Each `SignaturePackageRequire` carries the name, the optional version, an
   `exact` flag set when the call carried `-exact`, its source span, and a `conditional` flag set when the require sits inside a
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
   Both read through the server's closed-file store rather than `std::fs`:
   `scan_tree_in` / `scan_path_in` take a `tcl_lsp_core::vfs::SourceStore`,
   and the two-argument forms are `NativeStore` wrappers over them, so a
   browser host that supplies `pkgIndex.tcl` / `tclIndex` bytes builds the
   same database. See [lsp-source-store.md](lsp-source-store.md).
10. `parse_pkg_index` extracts `package ifneeded <name> <version> <script>`
    entries and the source targets the script would `source` / `load`;
    `parse_tcl_index` extracts the `auto_index` proc → file mappings from a
    `tclIndex`.
11. Index files are tokenised with the **real Tcl lexer**, so nested
    `[file join $dir x]`, quoted, and braced path forms are handled
    structurally — not by regex. The differential tests verify the output
    against a real `tclsh`.
12. `resolve_require(name, version, exact, prefer)` (and its
    `resolve(name, version)` shorthand, which takes the default `prefer`)
    returns the implementation files of the declaration C Tcl's
    `SelectPackage` would evaluate; `select_provider(…)` returns that whole
    declaration. The selection rule is
    `tcl_dialect::select_package_version`, a port of `generic/tclPkg.c`, and
    is the *only* version-comparison code in the workspace — the bytecode VM's
    `package vcompare` / `vsatisfies` / `require`, the `pkgIndex.tcl` guard
    evaluation and this resolver all go through it, so they cannot disagree.
    The rules, each pinned against `tclsh8.6` (8.6.14) and `tclsh9.0` (9.0.4),
    whose transcripts were byte-identical:

    | Requirement | Means | from providers 1.5 + 2.3 |
    |---|---|---|
    | *(none)* | anything | 2.3 — highest, **not** first discovered (#1090) |
    | `1.2` | `[1.2, 2)` — up to but excluding the next major | 1.5 |
    | `2.0` | `[2.0, 3)` | 2.3 |
    | `2.0-` | `[2.0, ∞)` | 2.3 |
    | `2.0-2.2` | `[2.0, 2.2)` — half-open | nothing |
    | `-exact 2.0` | the degenerate range `2.0-2.0` | nothing (#1090) |

    Two further rules the port implements:

    - **Prereleases.** `1.3b1` orders *above* `1.2` and *below* `1.3`, and
      satisfies the requirement `1.2` — but an unconstrained require prefers
      the highest **stable** candidate, so 1.2 + 1.3b1 selects 1.2. That is
      `package prefer`'s default on both interpreters.
    - **`-exact` is a range, not a separate code path.** `-exact V` is the
      requirement `V-V` (`tcl_dialect::exact_requirement`), the same rewrite
      `tclPkg.c` performs, and the degenerate range is the one form compared
      without the alpha padding — so `-exact 2.0` accepts `2.0.0` (it compares
      equal) and rejects `2.0a1`.

    Two further rules, both documented at
    `PackageResolver::resolve_require`:

    - **`package prefer` state is tracked per document** (#1126). `package
      prefer latest` raises the interpreter's selection mode, and
      `package_resolver::package_prefer_at(analysis, at)` reports the mode in
      force at a given `package require`, ordered by
      `indirection::in_effect` — the same "had this statement already run?"
      rule the import family uses. The state is a **monotone latch**: the
      default is `stable`, `package prefer stable` is a no-op from the default
      and silently ineffective after a raise, so "has a raise already run" is
      the whole state and only the raise is recorded
      (`AnalysisResult::package_prefer_latest`). A *conditional* raise and a
      raise in another document both abstain toward the default — the latter
      for the same reason every cross-file event abstains, no static load
      order.
    - **Two providers whose versions compare equal.** C Tcl collapses them
      into one `ifneeded` entry keeping the *first*'s version string and the
      *last*-sourced script, and the source order is `glob` order —
      filesystem order, machine-dependent. `select_provider` therefore keeps
      the first-discovered declaration (discovery sorts subdirectories by
      name), which is deterministic and is the one whose version string C Tcl
      reports; `resolve_require` returns the **union** of the equal-comparing
      providers' files, because which script survives is unknowable and
      naming every candidate file beats betting on a filesystem order.

    The comparator's per-pair oracle is pinned as a corpus, not regenerated at
    run time: `rust/tcl-dialect/tests/data/package_version_oracle.txt` (the
    full `package vcompare` / `package vsatisfies` grid plus the ill-formed
    inputs the interpreter rejects) and
    `rust/tcl-dialect/tests/data/package_select_oracle.txt` (multi-provider
    `package require` trials, including `-exact` and `package prefer`), both
    checked by `rust/tcl-dialect/tests/package_version_oracle.rs`.
13. `provides(name)` reports whether the scanned database knows `name`.
14. `transitive_available_packages(requires, read_fn)` returns the closure of
    packages the given requires pull in — following each `ifneeded` script's
    own `package require`s — which is how a wrapper package that internally
    `package require Tk` makes `Tk` available (#723).

### Declared package provides (#1813)

15. A **binary** extension can load another package with nothing in any Tcl
    source to say so: its C `Init` calls `Tcl_PkgRequire`, or it links Tk
    through `Tk_InitStubs`. Rule 14 has nothing to read there — a `load`-only
    `ifneeded` script sources no Tcl file — so the dependency is not
    discoverable by any scan, by us or by anything else. It is **declared**,
    in either of two places, because the two answer different questions.

16. **The directive** — `# tcl-lsp: package NAME provides PKG …` — states
    the edge for one file:

    ```tcl
    # tcl-lsp: package myExtension provides Tk
    package require myExtension
    ```

    `package` occupies the keyword slot, as in every other member of the
    family (`supports`, `stub`, `disable`), so `matches_marker` handles it and
    no future keyword can be confused with a package name; the wording
    deliberately echoes the Tcl commands the line is about.
    `parse_provides_directives` reads it from **anywhere** in the file (any
    line whose first non-whitespace character is `#`, the same terms
    `scan_source_for_stubs` reads a stub block on), because the line names
    its loader rather than relying on where it sits. It travels with the file
    and needs no configuration, so it is also the only form the CLI honours
    (`tcl diag` / `lint` read no `.tcl-lsp.ini`).

17. **The configured edge** — `[packages.provides]` in `.tcl-lsp.ini`, or
    `tclLsp.packages.provides` — says "requiring *this* package loads
    *those*", once for the whole project:

    ```ini
    [packages.provides]
    myExtension = Tk
    bigWrapper = Tk, Img
    ```

    equivalently `{ "myExtension": ["Tk"] }` (a bare string is accepted for
    one name).

18. Both reach the analyser as `Analyser::with_package_provides` /
    `directive_provides` and are applied by
    `expand_implied_package_requires` at the head of the shared diagnostic
    tail. Each package the edges reach becomes an ordinary
    `SignaturePackageRequire` on `AnalysisResult.package_requires`, carrying
    **the span of the `package require` that loaded it** — the position from
    which it is genuinely available, which is what keeps H301's ordering
    claim honest — and a `PackageRequireOrigin` (`Directive(pkg)` or
    `Provides(pkg)`, naming the loader) for the consumers that want what the
    source literally says. Every consumer that asks "is this package
    available here?" — W120, H301, the W123 widening, the Tk activation gate,
    completion's `tk_loaded`, hover's package annotation — therefore answers
    exactly as it would for a written `package require`, with no consumer
    taught about the mechanism.

    An edge is inert until the document requires its loader: a
    `# tcl-lsp: package myExtension provides Tk` in a file that never says
    `package require myExtension` declares nothing, which is what makes the
    two surfaces the same fact rather than two mechanisms.

19. The closure searches the file's own directives before the configured
    edges, and a package reached through either carries it onward, so the two
    surfaces chain. It is breadth-first over the **earliest anchor** each
    package has so far, not a visited set: a package the source *also*
    requires explicitly further down keeps the earlier, declared anchor,
    because H301 takes the minimum start per package and a set would leave
    only the later span — reporting the command between the two as
    used-before-available purely because the file names the package twice.
    Re-reaching a package at an anchor no earlier than the one it has stops,
    so a cyclic declaration still terminates. A declared entry carries no
    version requirement, so it contributes no version floor: a declaration
    says a package is *there*, never which release.

20. Every analyser that produces a **published or indexed** result carries
    the edges — the interactive path, the recovery path taken while a buffer
    has an unclosed delimiter, the disk scanner and the startup workspace
    scan. `workspace_index` harvests `package_requires`, and a `source`
    descendant inherits those names, so an analyser that omits the edges does
    not merely under-report its own file: it writes an index entry that
    under-reports for every file downstream of it. The edges are also
    resolved **per folder** (`tclLsp.packages.provides` is `scope: resource`
    and a `.tcl-lsp.ini` belongs to its own project), so a secondary root
    declares its own and never inherits a sibling's.

    The edges are one of the `"scope": "resource"` analyser inputs, and they
    all travel together in `ResourceAnalyserInputs` — `packages.provides`,
    `bigipVersion` and `targets`. Each was, at some point, wired to only a
    subset of the construction sites, so the bundle exists to make the *next*
    one a single edit rather than an audit: resolve with
    `Backend::resource_analyser_inputs` (a folder override wins field by
    field, else the global) or `ResourceAnalyserInputs::from_db_config` where
    a resolved salsa handle is already in hand, and hand it over with
    `apply`.

21. The tail is the one point both the whole-file walk and the per-item
    (incremental) walk reach with their `package_requires` merged, so the two
    strategies cannot disagree. The per-item path's mid-walk Tk hand-off runs
    *before* the tail, so `has_tk_require` consults the declarations directly
    (`tk_is_declared_available`), and `fresh_full_analyse` carries the
    configured edges into the full re-analysis it falls back to — the
    directives need no carrying, being a pure function of the source it
    re-scans.

### Workspace index integration

22. `WorkspaceIndex::add_document(uri, analysis)` records a document's procs,
    classes, invocation sites, `source` targets, and `package require`s;
    `remove_document(uri)` drops every entry from that document before a
    re-index or on close.
23. The server seeds the index from both editor-opened documents (via the
    diagnostics path) and an on-disk scan of the workspace folders
    (`scan_workspace_folders`), so unopened `.tcl` / `.tm` files are covered.
24. Open buffers win over disk-scanned copies: `merge_workspace_scan_results`
    re-checks the live open set at publication time and never overwrites an
    open document's entry with a stale on-disk analysis.

### Missing-`package require` refinement (W120)

25. The analyser's single-file W120 knows only the requires in the current
    document. Two workspace-level refinements are layered on top, both in the
    server's `refine_w120_diagnostics`:
    - **#723 transitive resolution** — a required package is resolved through
      the workspace `pkgIndex.tcl` database; a W120 for a package that the
      requires transitively provide is dropped. If any required package is
      *unknowable* (neither the registry nor the database knows it), it may
      load anything, so every W120 is conservatively dropped.
    - **#804 cross-file inheritance** — see below.

### Cross-file `package require` inheritance (W120, #804)

26. A file need not carry its own `package require` for a command whose package
    was required by an **entry** file that `source`s it.
27. **Automatic (default).** The workspace index's `source` targets and
    `package require`s feed a reverse-reachability walk
    (`tcl-lsp-core::source_graph::ancestor_requires`): a module inherits the
    requires of every file that transitively `source`s it. Only **literal**
    `source path.tcl` targets are followed; a computed `source $dir/x.tcl`
    yields no static edge. Path resolution is
    `source_graph::resolve_source_target` (lexical, no filesystem access); the
    server supplies the URI ↔ path conversion.
28. **Explicit.** `.tcl-lsp.ini [project] entryPoints` lists the entry files
    (relative to the folder root, or absolute). When set, the union of those
    files' requires is treated as available for W120 across the whole folder,
    and the automatic `source`-graph inheritance is **disabled** for that
    folder.
29. The inherited requires are merged with the document's own before the #723
    transitive resolution runs, so an inherited `package require myWrapper`
    still (transitively) pulls in whatever `myWrapper` provides.

### iRules cross-file equivalent

30. iRules do not support `package require` on BIG-IP, so W120 never applies
    to the `f5-irules` dialect (the refinement early-returns when the registry
    has no `package` command).
31. iRules procs are instead globally visible across files through the same
    workspace index proc aggregation the LSP cross-document features use.

### Split packages

32. A single `package ifneeded` script may `source` multiple files
    (`source [file join $dir a.tcl]; source [file join $dir b.tcl]`); each is
    extracted independently by the lexer-driven parser, so no regex
    semicolon-capture limitation applies.
33. A single `pkgIndex.tcl` may declare multiple unrelated packages — each
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
  `libraryPaths`, `[packages.provides]`).
- `tcl-compiler/src/analyser/state.rs` — `Analyser::with_package_provides`,
  `expand_implied_package_requires`, `implied_package_requires`.
- `tcl-compiler/src/analyser/utils.rs` — `parse_provides_directives`.
- `tcl-compiler/src/signature_scan/types.rs` — `PackageRequireOrigin`.

## Failure modes

- **Missing completions**: a `package require` not detected → the package's
  gated commands stay hidden. A **binary** extension that loads a package from
  its C `Init` is never detected by design; declare it with a
  `# tcl-lsp: provides` comment or under `[packages.provides]`.
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
- `tcl-compiler/src/analyser/tk_checks.rs` (tests) —
  `declared_package_provides_activates_tk`,
  `declared_package_provides_is_transitive_and_cycle_safe`,
  `declared_package_provides_agrees_across_walk_strategies`,
  `provides_directive_activates_tk`,
  `provides_directive_without_the_require_is_inert`,
  `provides_directive_and_configured_edge_compose`,
  `provides_directive_agrees_across_walk_strategies`.
- `tcl-compiler/src/analyser/utils.rs` (tests) —
  `parse_provides_directives_read_edges_from_the_whole_file`,
  `parse_provides_directives_ignore_other_shapes`.
- `tcl-lsp-server/src/lib.rs` (tests) — `refine_w120_*`,
  `declared_package_provides_makes_tk_available_to_diagnostics`,
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
