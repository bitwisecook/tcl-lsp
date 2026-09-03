# tclpkg — architecture overview

Tcl has no native way to declare, pin, fetch, verify, and reproduce
third-party package dependencies: `pkgIndex.tcl` covers discovery inside a
single `auto_path`, but there is no manifest, lockfile, or
content-addressable cache. `tclpkg` is a deterministic, MVS-based dependency manager integrated into the
`tcl` CLI (`rust/tcl-cli`) as the `tcl pkg …` and `tcl venv …` verb groups.
The engine is `rust/tcl-pkg`; the sandbox every external command runs under
is `rust/tcl-sandbox`. It draws design inspiration from Go modules (MVS
resolver, lockfile as source of truth), Zig (content-addressable cache keyed
by SHA-256), and Python's `venv` (virtual environment model).

```
tclpkg.tcl (manifest)
       │
       ▼
  Whitelisted directive parser  ──────────►  Manifest
  (manifest.rs — no interpreter)                  │
                                                  ▼
                                    MVS resolver (resolver.rs)
                                                  │
                                                  ▼
                                    Content-addressable store
                                    (cas.rs, <cache_dir>/tclpkg/)
                                                  │
                                                  ▼
                                    tclpkg.lock (lockfile.rs)
                                                  │
                                                  ▼
                              Materialise → ./lib/<pkg>-<ver>/
                              or            .venv/lib/<pkg>-<ver>/
```

## Decision rules / contracts

### Manifest (`tclpkg.tcl`)

1. The manifest is **pure data**. No interpreter runs it: `manifest.rs`
   parses the file into commands and words using Tcl grouping rules
   (braces, quotes, backslashes) and never performs variable or command
   substitution, so no package-provided code can execute as a side effect
   of resolving or installing.
2. Fourteen directives are permitted: `package`, `version`, `description`,
   `license`, `author`, `homepage`, `tcl`, `require`, `dev-require`,
   `replace`, `exclude`, `provides`, `entry`, `build`. Anything else is
   refused with `command not permitted in safe mode: <cmd>`.
3. `package` and `version` are required; every other directive is optional,
   and a repeated `package` or `version` is an error.
4. The `tcl` constraint defaults to `>=8.6` when omitted.
5. `build` declares a build script but never causes one to run — see
   [`tclpkg-security.md`](tclpkg-security.md).

### Lockfile (`tclpkg.lock`)

6. Canonical JSON, emitted through `json.rs`: recursively sorted object
   keys, 2-space indent, `": "` key separator, LF endings, final newline,
   `packages[]` sorted by `(name, version)`.
7. Two invocations against the same manifest and registry snapshot produce
   byte-identical output. Only the `generated` timestamp varies, and
   `--frozen` preserves it.
8. The schema version is bumped only on a breaking change.
9. The lockfile records the exact source, integrity hash, size, and the
   `provides` / `license` read back from each fetched package.

### MVS resolver

10. Minimum Version Selection is deterministic and solverless: every
    `require <pkg> <minver>` declares a minimum, and the resolver picks the
    maximum-of-minimums for each package across the whole graph. No upper
    bounds, no backtracking, no SAT solver.
11. `replace` from the root manifest forces a specific version; a
    transitive `replace` is ignored.
12. `exclude` from the root manifest refuses a specific version and errors
    with the dependency chain that selected it.
13. The resolver is a pure function over data — callers supply the provider
    mapping — so it is testable without network or filesystem.

### Version ordering

14. `version.rs` agrees with both semver 2.0 and C Tcl's `package require`
    ordering: missing patch digits default to zero, a leading `v` is
    tolerated, and Tcl-style `a1` / `b2` / `rc1` prereleases collate
    alongside semver's `-alpha.1` form (`8.6.3a1` sorts before `8.6.3`).

### Content-addressable cache

15. Entries live at
    `<cache_dir>/tclpkg/cas/sha256/<ab>/<full_hash>/tree/` and are
    immutable once written.
16. The integrity string is `sha256-<base64url-no-pad>`.
17. The hash is computed over a canonicalised worktree: sorted paths,
    stripped VCS directories, permission-masked, timestamps ignored — so a
    re-compressed archive of the same tree hashes identically.
18. Archive extraction (`fetchers.rs`) is hardened against zip-slip,
    absolute paths, symlinks, and decompression bombs.

### Registry client

19. `registry.rs` fetches the upstream `packages.json` discovery metadata,
    caches it under `<cache_dir>/tclpkg/registry/`, and respects a 24-hour
    TTL with conditional GET (`If-None-Match` / `304 Not Modified`).
20. Offline mode reads the cache only and never touches the network.

### Virtual environments

21. `tcl venv create .venv` produces `bin/`, `lib/`, and a config file.
22. Activation prepends `<venv>/bin` to `$PATH` and sets `$TCLLIBPATH`; the
    `bin/tclsh` wrapper sets `TCLLIBPATH` itself, so non-interactive use
    works without activation.
23. Activation scripts are provided for bash/zsh and fish.

### Source discovery

24. `tcl pkg discover` is read-only by default and never accesses the network.
    It inventories `package require` through the full registry-dispatched
    analyser, then uses the optimiser's constant propagation and pure builtin
    folds to refine dynamic words while retaining original file/line
    provenance.
25. `discover --add` changes only `tclpkg.tcl`. It adds deterministic,
    unconditional requirements which the manifest's minimum-version model can
    represent; ambiguous, guarded, exact, and bounded requirements remain
    review findings. Installed, vendored, virtual-environment, build, and
    generated trees are not scanned as direct project source.

## CLI surface

`tcl pkg` — `init`, `discover`, `install`, `list`, `tree`, `verify`, `info`, `add`,
`remove`, `update`, `sync`, `outdated`, `why`, `vendor`, `run`, `freeze`,
`search`, `show`, plus the security verbs `policy`, `hooks`, `audit`,
`trust`, and `build` documented in
[`tclpkg-security.md`](tclpkg-security.md).

`tcl venv` — `create`, `delete`, `info`, `activate`, `deactivate`, `list`.

`ui.rs` centralises the output conventions every subcommand shares: ANSI
colour, the check/cross/warning symbols, and the canonical `--json` mode.

## File-path anchors

- `rust/tcl-pkg/src/lib.rs` — public API surface
- `rust/tcl-pkg/src/manifest.rs` — whitelisted-directive manifest parser
- `rust/tcl-pkg/src/lockfile.rs` — lockfile I/O
- `rust/tcl-pkg/src/json.rs` — canonical JSON emitter
- `rust/tcl-pkg/src/resolver.rs` — MVS resolver
- `rust/tcl-pkg/src/version.rs` — version type and ordering
- `rust/tcl-pkg/src/cas.rs` — CAS and integrity hashing
- `rust/tcl-pkg/src/fetchers.rs` — tarball / git / path fetchers
- `rust/tcl-pkg/src/installer.rs` — resolve → fetch → store → materialise
- `rust/tcl-pkg/src/registry.rs` — registry client and TTL cache
- `rust/tcl-pkg/src/venv.rs` — virtual environment management
- `rust/tcl-pkg/src/docker.rs` — Dockerfile generation for Tcl projects
- `rust/tcl-pkg/src/exec.rs` — the single external-execution chokepoint
- `rust/tcl-pkg/src/policy.rs`, `hooks.rs` — operator policy and lifecycle hooks
- `rust/tcl-pkg/src/ui.rs` — CLI output helpers
- `rust/tcl-pkg/src/errors.rs` — error types; the CLI prints `Display` verbatim
- `rust/tcl-cli/src/commands/pkg.rs`, `venv.rs` — verb handlers
- `rust/tcl-cli/src/cli.rs` — the `pkg` / `venv` subcommand definitions

## Failure modes

1. **Manifest parse error** — reported with a file:line location prefix.
2. **Resolution failure** — reported with the dependency chain that
   selected the offending version.
3. **Integrity mismatch** — reported with expected and actual hash.
4. **Network failure** — falls back to the cached registry; errors when
   there is no cache.
5. **tclsh not found** — errors with a hint to install one or pass
   `--tcl`.

Every error's `Display` form is printed verbatim by the CLI, so the message
composition (location prefixes, `hint:` suffixes, integrity detail) is part
of the contract.

## Discoverability

- [tclpkg security architecture](tclpkg-security.md) — the sandbox,
  operator hooks, and locked-down policy layered over this engine.
- [package-loading contract](contracts/package-loading.md) — the
  pre-existing `pkgIndex.tcl` loading path.
- [xdg-config contract](contracts/xdg-config.md) — the XDG config and cache
  paths.
- [tcl verb CLI feature](../kcs/features/kcs-feature-tcl-verb-cli.md) — the
  `tcl` CLI contracts.
