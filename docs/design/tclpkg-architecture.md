# tclpkg — architecture overview

## Symptom

Tcl projects lack a native way to declare, pin, fetch, verify, and
reproduce third-party package dependencies.  `pkgIndex.tcl` covers
discovery inside a single `auto_path`, but there is no manifest,
lockfile, or content-addressable cache.

## Operational context

`tclpkg` is a deterministic, MVS-based dependency manager integrated into
the `tcl` CLI (`tooling/tcl/main.py`) via `tcl pkg …` and `tcl venv …`
verb groups.  It draws design inspiration from Go modules (MVS resolver,
lockfile as source of truth), Zig (content-addressable cache keyed by
SHA-256), and Python venv (virtual environment model).

### Architecture overview

```
tclpkg.tcl (manifest)
       │
       ▼
  Sandboxed TclInterp  ──────────►  ManifestAST
  (vm/interp.py safe mode)              │
                                        ▼
                               MVS Resolver (tooling/tclpkg/resolver.py)
                                        │
                                        ▼
                              Content-Addressable Store
                              (tooling/tclpkg/cas.py, ~/.cache/tcl-lsp/tooling/tclpkg/)
                                        │
                                        ▼
                               tclpkg.lock (lockfile)
                                        │
                                        ▼
                          Materialise → ./lib/<pkg>-<ver>/
                          or            .venv/lib/<pkg>-<ver>/
```

## Decision rules / contracts

### Manifest (`tclpkg.tcl`)

1. Evaluated in a sandboxed `TclInterp(safe=True)` with a whitelist of
   13 directives: `package`, `version`, `description`, `license`,
   `author`, `homepage`, `tcl`, `require`, `dev-require`, `replace`,
   `exclude`, `provides`, `entry`.
2. Any command not on the whitelist is refused at the INVOKE level.
3. `package` and `version` are required; all others are optional.
4. `tcl` constraint defaults to `>=8.6` if omitted.
5. Implemented in `tooling/tclpkg/manifest.py`.

### Lockfile (`tclpkg.lock`)

6. Canonical JSON: sorted keys, 2-space indent, LF endings, final newline.
7. Two invocations against the same manifest + registry produce
   byte-identical output (except the `generated` timestamp, preserved by
   `--frozen`).
8. Schema version (`"version": 1`) bumped only on breaking changes.
9. Implemented in `tooling/tclpkg/lockfile.py`.

### MVS Resolver

10. BFS walk picks max-of-minimums for each package.
11. `replace` from the root manifest forces a specific version; transitive
    `replace` is ignored.
12. `exclude` from the root manifest refuses a specific version; errors
    with the dependency chain that selected it.
13. Implemented in `tooling/tclpkg/resolver.py`.

### Content-addressable cache

14. Location: `~/.cache/tcl-lsp/tooling/tclpkg/cas/sha256/<ab>/<hash>/tree/`.
15. Integrity string format: `sha256-<base64url-no-pad>`.
16. Hash computed over canonicalised worktree (sorted paths, stripped
    `.git/`, permission-masked, timestamps ignored).
17. Entries are immutable once written.
18. Implemented in `tooling/tclpkg/cas.py`.

### Virtual environments

19. `tcl venv create .venv` produces `bin/`, `lib/`, `tclvenv.cfg`.
20. `bin/tclsh` wrapper always sets `TCLLIBPATH`.
21. Activation scripts for bash/zsh and fish.
22. Implemented in `tooling/tclpkg/venv.py`.

### LSP integration

23. `_KNOWN_TCL_LSP_SECTIONS` includes `"packageManager"`.
24. `tcl-lsp.tclpkg.install` adds missing manifest requirements, rewrites
    `tclpkg.lock`, and returns a status dict; `tcl-lsp.tclpkg.search`
    searches the offline registry cache.
25. W120 code action offers "Install '<pkg>' via tclpkg" alongside the
    existing "Add 'package require'" action.
26. VS Code settings under `tclLsp.packageManager.*`.

## File-path anchors

- `tooling/tclpkg/__init__.py` — public API surface
- `tooling/tclpkg/manifest.py` — manifest loader
- `tooling/tclpkg/lockfile.py` — lockfile I/O
- `tooling/tclpkg/resolver.py` — MVS resolver
- `tooling/tclpkg/cas.py` — CAS + integrity hashing
- `tooling/tclpkg/fetchers.py` — tarball/git/path fetchers
- `tooling/tclpkg/registry.py` — registry client
- `tooling/tclpkg/venv.py` — virtual environment management
- `tooling/tclpkg/ui.py` — CLI output helpers
- `tooling/explorer/verbs/pkg.py` — `tcl pkg` CLI verb handlers
- `tooling/explorer/verbs/venv.py` — `tcl venv` CLI verb handlers
- `tooling/tooling/vm/interp.py:102` — `TclInterp(safe=…)` parameter
- `tooling/tooling/vm/commands/interp_cmds.py:65` — `interp issafe` handler
- `shared/user_config.py:126` — `_cache_dir()` helper
- `server/settings.py:58` — `_KNOWN_TCL_LSP_SECTIONS`
- `server/commands.py:825` — `tcl-lsp.tclpkg.install` command handler
- `server/features/code_actions.py:383` — `_tclpkg_install_action()`

## Failure modes

1. **Manifest parse error** — `ManifestError` with file:line location.
2. **Resolution failure** — `ResolutionError` with the dependency chain.
3. **Integrity mismatch** — `IntegrityError` with expected vs actual hash.
4. **Network failure** — falls back to cached registry; errors if no cache.
5. **tclsh not found** — `VenvError` with hint to install or specify `--tcl`.

## Test anchors

- `tests/tooling/tclpkg/test_manifest.py` — 29 tests for manifest parsing
- `tests/tooling/tclpkg/test_lockfile.py` — 23 tests for lockfile serialisation
- `tests/tooling/tclpkg/test_cas.py` — 18 tests for CAS hashing + storage
- `tests/tooling/tclpkg/test_resolver.py` — 15 tests for MVS resolution
- `tests/tooling/tclpkg/test_version.py` — 34 tests for version ordering
- `tests/test_vm_safe_mode.py` — 12 tests for VM safe-mode

## Discoverability

- [package-loading contract](contracts/package-loading.md) — pre-existing `pkgIndex.tcl` loading
- [xdg-config contract](contracts/xdg-config.md) — XDG config paths (see also `_cache_dir`)
- [tcl verb CLI feature](../kcs/features/kcs-feature-tcl-verb-cli.md) — `tcl` CLI contracts
