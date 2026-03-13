# KCS: Package loading contracts

## Symptom

Completions, hover, or diagnostics are missing for package-gated commands
(stdlib, tcllib, Tk), or cross-file navigation silently drops procs from
resolved packages. iRules procs from other files are not found.

## Operational context

The package loading system sits between the analyser (which detects
`package require` statements) and the LSP features (which filter commands
based on active packages). It spans four layers:

1. **Analyser** — extracts `PackageRequire` from source text.
2. **Command registry** — gates commands via `required_package` / `tcllib_package`.
3. **Package resolver** — scans `pkgIndex.tcl` files to find implementation sources.
4. **Workspace index** — stores procs from resolved package files with `EntrySource.PACKAGE`.

Three package sources are supported, plus the iRules cross-file equivalent:

| Source  | `required_package` value | Registration location                         |
|---------|--------------------------|-----------------------------------------------|
| Stdlib  | e.g. `"http"`, `"msgcat"` | `core/commands/registry/stdlib/`               |
| Tcllib  | derived from `tcllib_package` | `core/commands/registry/tcllib/`            |
| Tk      | `"Tk"`                   | `core/commands/registry/tk/`                   |
| iRules  | n/a (no packages on BIG-IP) | `lsp/workspace/workspace_index.py` globals  |

## Decision rules / contracts

### Analyser extraction

1. `package require <name>` and `package require -exact <name> <version>` are
   recorded as `PackageRequire` in `AnalysisResult.package_requires`.
2. `package provide` and `package ifneeded` are **not** tracked as requires.
3. `AnalysisResult.active_package_names()` returns a `frozenset[str]` of all
   required package names — this is the per-document filter key.
4. Version is captured but currently only used by the package resolver, not
   by registry filtering.

### Registry filtering

5. `CommandSpec.supports_packages(active_packages)` returns `True` when:
   - `required_package is None` (unconditional command), or
   - `active_packages is None` (no filtering), or
   - `required_package in active_packages`.
6. `active_packages=frozenset()` (empty set) hides all package-gated commands
   — only core builtins remain.
7. `active_packages=None` shows everything — backwards-compatible default.
8. Tcllib specs set `tcllib_package`; during registry build this is
   promoted to `required_package` so the same filtering path applies.
9. Tk specs set `required_package="Tk"` and `warn_missing_import=False`
   because `wish` auto-loads Tk.

### Package resolver (pkgIndex.tcl)

10. Searches all directories under configured search paths for
    `pkgIndex.tcl` files via `os.walk`.
11. Parses `package ifneeded <name> <version> <script>` lines.
12. Extracts source file references from the script via two patterns:
    - `source [file join $dir <filename>]` — `_SOURCE_JOIN_RE`
    - `source $dir/<filename>` — `_SOURCE_DIR_RE`
13. **Fallback**: if no source patterns matched, returns all `.tcl` files
    in the directory (excluding `pkgIndex.tcl` itself).
14. Version matching: exact match or prefix match (e.g. `"2"` matches `"2.9"`).
15. Lazy scanning: `scan_packages()` is called automatically on first
    `resolve()` or `all_package_names()` if not yet scanned.
16. `configure()` resets the scanned state — subsequent calls rescan.

### Workspace index integration

17. Resolved package files are indexed with `EntrySource.PACKAGE`.
18. `is_background()` returns `True` for both `BACKGROUND` and `PACKAGE` entries.
19. `remove_background_entries()` clears both `BACKGROUND` and `PACKAGE` entries
    but preserves `OPEN` entries.
20. Opening a file (`EntrySource.OPEN`) overrides its `PACKAGE`/`BACKGROUND` status.

### iRules cross-file equivalent

21. iRules do not support `package require` on BIG-IP.
22. Instead, all procs from iRules files are globally visible via
    `WorkspaceIndex.update_irules_globals()`.
23. Cross-file variables are shared via RULE_INIT blocks and tracked in
    `WorkspaceIndex._irules_rule_init_vars`.
24. `remove_background_entries()` also clears iRules globals.

### Split packages

25. A single `package ifneeded` script may reference multiple source files
    (e.g. `source [file join $dir a.tcl]; source [file join $dir b.tcl]`).
26. The `_SOURCE_JOIN_RE` regex matches each `source [file join ...]`
    occurrence in the script independently.
27. **Known limitation**: the `$dir/` pattern (`_SOURCE_DIR_RE`) captures
    trailing semicolons in filenames when multiple references appear on one
    line. Only the last file (with no trailing `;`) will typically pass
    `os.path.isfile`, so `_extract_source_files()` resolves just that file
    and the others are effectively ignored. In this situation the resolver
    does *not* fall back to the directory-listing heuristic; fallback only
    occurs when **no** `$dir/...` candidates match.
28. A single `pkgIndex.tcl` may declare multiple unrelated packages — each
    `package ifneeded` line is parsed independently.

## File-path anchors

- `core/analysis/semantic_model.py` — `PackageRequire`, `active_package_names()`
- `core/analysis/analyser.py` — package require extraction (~line 876)
- `core/commands/registry/models.py` — `required_package`, `tcllib_package`, `supports_packages()`
- `core/commands/registry/command_registry.py` — package filtering methods
- `core/commands/registry/stdlib/` — stdlib command specs
- `core/commands/registry/tcllib/` — tcllib command specs
- `core/commands/registry/tk/` — Tk command specs
- `core/tk/detection.py` — `has_tk_require()`
- `core/packages/resolver.py` — `PackageResolver`, pkgIndex parsing
- `lsp/workspace/workspace_index.py` — `EntrySource`, iRules globals
- `lsp/features/package_suggestions.py` — `rank_package_suggestions()`
- `lsp/server.py` — server-side resolve-and-index orchestration

## Failure modes

- **Missing completions**: `package require` not detected → `active_packages` empty → all package-gated commands hidden.
- **False W120 diagnostics**: analyser fails to record a package require → W120 fires for commands that are actually imported.
- **stale package index**: `configure()` not called after search path change → resolver returns old state.
- **Split-file partial resolution**: `$dir/` pattern with semicolons misses files → some procs from the package not indexed.
- **iRules proc not found**: `update_irules_globals()` not called for a file → its procs invisible to other iRules files.
- **PACKAGE entries persist after close**: `remove_background_entries()` not called → stale procs from old packages linger.

## Test anchors

- `tests/test_package_loading.py` — comprehensive cross-cutting tests
- `tests/test_package_resolver.py` — pkgIndex parsing and resolution
- `tests/test_tcllib.py` — tcllib registry, completion, hover, diagnostics
- `tests/test_stdlib_registry.py` — stdlib registry and filtering
- `tests/test_tk_registry.py` — Tk registry and commands_for_packages
- `tests/test_tk_detection.py` — `has_tk_require()` detection
- `tests/test_vm_package.py` — VM-level package command implementation
- `tests/test_package_suggestions.py` — suggestion ranking
- `tests/test_workspace_index.py` — workspace index and entry source tracking

## Discoverability

- [KCS index](README.md)
- [Workspace indexing contracts](kcs-workspace-indexing-contracts.md)
- [Command registry and event model](kcs-command-registry-event-model.md)
- [LSP feature providers](kcs-lsp-feature-providers.md)
- [LSP diagnostics publication](kcs-lsp-diagnostics-publication.md)
