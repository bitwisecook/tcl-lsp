# tclpkg contracts — manifest, resolver, lockfile, CAS, venv, policy

The project-local package manager, `rust/tcl-pkg`, driven by the `tcl pkg` and
`tcl venv` CLI verbs. This document is the contract layer: the invariants each
subsystem guarantees and where they are implemented. The architecture narrative
lives in [`../tclpkg-architecture.md`](../tclpkg-architecture.md) and the threat
model in [`../tclpkg-security.md`](../tclpkg-security.md); the user-facing
guide is
[`kcs-howto-manage-tcl-packages.md`](../../kcs/kcs-howto-manage-tcl-packages.md).

Not to be confused with [package-loading.md](package-loading.md), which is
about how the *analyser* accounts for `package require` — a different subsystem
entirely.

## Manifest — `tclpkg.tcl` (`manifest.rs`)

1. The manifest is pure data: no variable or command substitution. It is parsed
   into commands and words using Tcl grouping rules (braces, quotes,
   backslashes, comments, semicolons) and each directive is dispatched
   directly — no interpreter is instantiated.
2. Thirteen directives are permitted: `package`, `version`, `description`,
   `license`, `author`, `homepage`, `tcl`, `require`, `dev-require`,
   `replace`, `exclude`, `provides`, `entry`. Any other command is refused
   with `command not permitted in safe mode: <cmd>`, as a safe interpreter
   would. `manifest::directive_names()` is the canonical list, cross-checked
   against the registry's `TCLPKG_MANIFEST_ENV` scoped environment by a drift
   test (`rust/tcl-pkg/tests/manifest_env_drift.rs`).
3. `package` and `version` are required; everything else is optional. A
   duplicate `package` directive is rejected immediately, as is the same
   package appearing in both `require` and `dev-require`.
4. `version` is semver 2.0 with Tcl-style pre-release spellings (`a1`, `b2`,
   `rc1`). The `tcl` constraint defaults to `>=8.6`.
5. `require` / `dev-require` take `name minver ?-source URL?`.
6. `replace` and `exclude` are honoured **from the root manifest only**;
   transitive occurrences are ignored.

## Resolver — MVS (`resolver.rs`)

7. Go-style Minimum Version Selection: a BFS over the dependency graph picking
   the maximum-of-minimums per package. No upper bounds, no backtracking, no
   SAT solver.
8. The resolver is a pure function over data — the caller supplies a provider
   mapping `(name, version)` to that package's own requirements — so it is
   testable without any network or filesystem.
9. `replace` forces a version for a named package; `exclude` refuses an exact
   `(name, version)` pair, and if MVS would have selected it the error names
   the chain that selected it.
10. A convergence pass re-processes packages whose minimums were bumped after
    they were first processed. An iteration cap is the safety valve against a
    pathological graph.
11. Dev dependencies are included by default; `--no-dev` excludes them. The
    result is sorted by package name.

## Lockfile — `tclpkg.lock` (`lockfile.rs`)

12. Canonical JSON: alphabetically sorted keys, 2-space indent, LF endings,
    final newline. `packages[]` is sorted by `(name, version)`.
13. Two invocations against the same manifest and registry snapshot produce
    **byte-identical** output; the `generated` timestamp is the only
    non-deterministic field, and `--frozen` preserves the existing one (and
    refuses any other change).
14. `integrity` is `sha256-<base64url-no-pad>` (SRI-compatible).
15. `version` (`LOCKFILE_VERSION`, currently 1) is bumped only on an
    incompatible change; a lockfile declaring a newer schema is refused with a
    `TclPkgError` carrying an upgrade hint.
16. Writes are atomic — temp file plus rename — so a crash never leaves a
    partial lockfile.

## Content-addressable store (`cas.rs`)

17. The CAS lives at `<cache_dir>/tclpkg/cas/sha256/<ab>/<hash>/tree/`, where
    `<ab>` is the first two hex characters of the digest (sharding).
18. The hash is computed over the **canonicalised worktree**, not raw archive
    bytes, so a re-compressed archive hashes identically. Canonicalisation
    strips VCS and platform noise (`.git`, `.hg`, `.svn`, `.fossil`,
    `.DS_Store`, `Thumbs.db`, plus any `.tclpkgignore` entries), sorts files by
    POSIX path bytes, and folds in path, mode, size, and per-file content hash.
19. Timestamps, uid, gid, and xattrs are deliberately ignored, for
    cross-machine stability. Directory symlinks are not followed.
20. Entries are immutable once written: a later `store()` of the same hash is a
    no-op.
21. Materialisation into `lib/` uses symlinks by default, falling back to copy
    on platforms that restrict them.

## Virtual environments (`venv.rs`)

22. `tcl venv create .venv` produces `bin/`, `lib/`, and `tclvenv.cfg`.
23. `bin/tclsh` is a POSIX shell wrapper that always sets `TCLLIBPATH` before
    exec-ing the pinned `tclsh`, so non-interactive use works with no
    activation step.
24. Activation scripts for bash/zsh (`bin/activate`) and fish
    (`bin/activate.fish`) set `TCLLIBPATH`, `PATH`, `TCL_VENV`, and the prompt;
    `deactivate` restores the saved environment.
25. `tclvenv.cfg` records the Tcl version, the `tclsh` executable, the prompt
    label, whether system site packages are included, and the project root.
26. `tcl venv update --tcl VERSION` rewrites the wrapper and config without
    touching `lib/`. `tcl venv delete` refuses to delete the currently-active
    venv without `--force`.
27. `tclsh` discovery is `venv::find_tclsh()`, which prefers newer versions
    (`tclsh9.0`, `tclsh8.6`, `tclsh8.5`, `tclsh8.4`, `tclsh`).

## Registry client and policy

28. `registry.rs` fetches the upstream `packages.json` discovery metadata,
    caches it under `<cache_dir>/tclpkg/registry/`, and honours a 24-hour TTL
    with conditional GET (`If-None-Match` / `304`). Offline mode reads the
    cache only and never opens a socket.
29. `policy.rs` merges three TOML layers, lowest precedence first: system
    (`/etc/tcl-lsp/pkg-policy.toml`, `%PROGRAMDATA%\tcl-lsp\…` on Windows),
    user (`~/.config/tcl-lsp/pkg.toml`), project (`tclpkg.toml` beside the
    manifest). The system layer is honoured only when the file is owned by
    root/Administrators and is not world-writable, so a developer cannot
    weaken it by editing a file they own, and it may **lock** individual keys
    against the layers above it.

## Editor integration — the current gap

There is no LSP-side tclpkg integration today. Diagnostic codes **W130–W134**
are registered as *reserved* in `rust/tcl-core-types/src/diag_code.rs`
(`diag_reserved(Tclpkg, …)`) — they have text and are wired into the tables,
but nothing emits them, and the code-table test asserts exactly that. Likewise
the VS Code `tclLsp.packageManager.*` settings
(`registryUrl`, `cacheDir`, `autoInstallOnSave`, `offline`) are contributed by
the extension but no server-side handler reads them, and there is no
`tcl-lsp.tclpkg.install` / `…search` `executeCommand` handler.

Wiring any of that up means emitting the reserved codes from a real producer
and removing them from the reserved list in the same change.

## Key files

| Area | File |
|---|---|
| Manifest | `rust/tcl-pkg/src/manifest.rs` |
| Resolver | `rust/tcl-pkg/src/resolver.rs`, `version.rs` |
| Lockfile | `rust/tcl-pkg/src/lockfile.rs`, `json.rs` |
| CAS + integrity | `rust/tcl-pkg/src/cas.rs` |
| Fetchers / installer | `rust/tcl-pkg/src/fetchers.rs`, `installer.rs`, `hooks.rs` |
| Registry client | `rust/tcl-pkg/src/registry.rs` |
| Policy | `rust/tcl-pkg/src/policy.rs` |
| Virtual environments | `rust/tcl-pkg/src/venv.rs` |
| Docker generation | `rust/tcl-pkg/src/docker.rs` |
| CLI verbs | `rust/tcl-cli/src/commands/pkg.rs`, `venv.rs`, `rust/tcl-cli/src/cli.rs` |
| CLI tests | `rust/tcl-cli/tests/pkg_verbs.rs` |
