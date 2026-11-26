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
2. Fifteen directives are permitted: `package`, `version`, `description`,
   `license`, `author`, `homepage`, `tcl`, `require`, `dev-require`,
   `replace`, `exclude`, `provides`, `entry`, `build`, `spec`. Anything else
   is refused with `command not permitted in safe mode: <cmd>`.
3. `package` and `version` are required; every other directive is optional,
   and a repeated `package` or `version` is an error.
4. The `tcl` constraint defaults to `>=8.6` when omitted.
5. `build` declares a build script but never causes one to run — see
   [`security.md`](security.md).
5a. `spec { packs {a.tclspec b.tclspec} tier direct }` declares the
   `.tclspec` packs the package ships and the tier it asks for them at. Like
   `build` it is data: the braces hold key and value words, never a script.
   Each pack is a relative path to a `.tclspec` file inside the package
   directory (no `..`, no absolute or drive-qualified path), and `tier` is
   `root`, `direct`, `transitive` or `development`, `direct` when omitted. The
   pack loader reads the directive when it discovers packs beside a manifest: a
   manifest with one has those packs and no others, and one without keeps every
   `.tclspec` under its directory. The tier is a request and never a claim:
   `tier::clamp_requested` holds it to be no nearer the root than the position
   the lockfile's graph gives the package, so a package may ask for less than
   its position licenses and never more, and the workspace's own package takes
   no request.

### Lockfile (`tclpkg.lock`)

6. Canonical JSON, emitted through `json.rs`: recursively sorted object
   keys, 2-space indent, `": "` key separator, LF endings, final newline,
   `packages[]` sorted by `(name, version)`.
7. Two invocations against the same manifest and registry snapshot produce
   byte-identical output. Only the `generated` timestamp varies, and
   `--frozen` preserves it.
8. The schema version is bumped only on a breaking change.
9. The lockfile records the exact source, integrity hash, size, and the
   `provides` / `license` read back from each fetched package. A package whose
   manifest has a `spec` directive also records `spec_integrity`: the content
   hash of each pack it names, as `xxh3-` and sixteen hex digits, joined by
   commas in the manifest's order. Each is the value a compiled unit's claim on
   that pack carries (`tcl_spectcl::package_specs::pack_file_hash`, folded over
   any fragment the pack includes), so the lockfile and an artefact identify a
   pack by one hash, and a changed pack in an unchanged package release changes
   the lockfile. A package that names no packs has no such field.

### MVS resolver

10. A plain `require <pkg> <minver>` declares a minimum. The resolver picks
    the greatest minimum when a package has only plain minimums. Tcl operands
    such as `{-tcl 1.2-2.0 3.0}` or `{-exact 2.0}` retain Tcl alternatives,
    upper bounds and equality. The resolver picks the lowest written
    candidate that satisfies all the package's constraints, using Tcl's
    package version owner for Tcl comparisons. The source registry supplies
    locations, so the candidate set comes from the dependency graph.
11. `replace` from the root manifest forces a specific version subject to
    Tcl operands. A transitive `replace` is ignored.
12. `exclude` from the root manifest refuses a specific version and errors
    with the dependency chain that selected it.
13. The resolver is a pure function over data — callers supply the provider
    mapping — so it is testable without network or filesystem.

### Version ordering

14. tclpkg release tags and plain minimums use the semver-compatible
    `Version` type. Tcl dependency operands use the shared Tcl package
    comparison and satisfaction functions in `tcl-dialect`.

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
    unconditional requirements under their original Tcl version constraints.
    Unresolved words and guarded or loop-contained calls remain review
    findings. Installed, vendored, virtual-environment, build, and generated
    trees are outside the direct project source inventory.

## SpecTcl facts and the analysis

Three parts of tclpkg read command facts or feed the analysis. Each moves
onto the target interface as consumer 14 of the
[consumer migration](../registry/model/319-consumer-migration.md#the-consumers)
(wave 7, after SpecTcl 3's images, items 305, 307 and 406):

| Part | Today | Target |
|---|---|---|
| `tcl docker create` | loads the project's packs with `tcl_spectcl::pack::load` and reads each command's `runtime_backing` and `required_package` (`rust/tcl-cli/src/commands/docker.rs:58-84`) | reads `runtime_backing` through a `FactView` over the project's packs, and the providing package from the library's image; SpecTcl 3.0 has no `required_package` (`available P 0-` replaces it) |
| `tcl pkg discover` (rule 24) | runs `Analyser` and the optimiser itself over `tcl_cli_support::registry_for_dialect` (`rust/tcl-cli/src/commands/pkg_discover.rs:31-33, 214, 258`) | reads `DocumentFacts` from `tcl-db`'s session-free batch constructor, as MCP and the CLI do ([293](../workspace/293-mcp.md#analysis-without-a-session)) |
| `tclpkg.lock` and the `spec` tier (rule 5a) | the tier clamp reads the lockfile's graph when packs are discovered | the lockfile is a `tcl-db` input: the server's `PackState` watches it, and the CLI passes it to the batch constructor, so a lockfile edit re-tiers the packs and re-runs only what reads them |

The lockfile and the image agree on a pack's identity: `spec_integrity` (rule
9) is the hash a compiled image records as its `source_hash`.

## CLI surface

`tcl pkg` — `init`, `discover` (alias `scan`), `install`, `list`, `tree`,
`verify`, `info`, `add`, `remove`, `update`, `sync`, `outdated`, `why`,
`vendor`, `run`, `freeze`, `search`, plus the security verbs `policy`,
`hooks`, `audit`, `trust`, and `build` documented in
[`security.md`](security.md).

`tcl venv` — `create`, `delete`, `info`, `activate`, `deactivate`, `list`,
`update`, `run`.

`ui.rs` centralises the output conventions every subcommand shares: ANSI
colour, the check/cross/warning symbols, and the canonical `--json` mode.

## File-path anchors

- `rust/tcl-pkg/src/lib.rs` — public API surface; re-exports the data-model
  modules below
- `rust/tcl-pkg-model/src/manifest.rs` — whitelisted-directive manifest parser
- `rust/tcl-pkg-model/src/lockfile.rs` — lockfile I/O
- `rust/tcl-pkg-model/src/json.rs` — canonical JSON emitter
- `rust/tcl-pkg-model/src/tier.rs` — a package's dependency tier, from the
  lockfile's graph and the root manifest's requirements, and the clamp a
  `spec` directive's requested tier is held to
- `rust/tcl-spectcl/src/package_specs.rs` — the packs a `spec` directive names
  and the hash the lockfile records for them
- `rust/tcl-pkg/src/resolver.rs` — MVS resolver
- `rust/tcl-pkg-model/src/version.rs` — version type and ordering
- `rust/tcl-pkg/src/cas.rs` — CAS and integrity hashing
- `rust/tcl-pkg/src/fetchers.rs` — tarball / git / path fetchers
- `rust/tcl-pkg/src/installer.rs` — resolve → fetch → store → materialise
- `rust/tcl-pkg/src/registry.rs` — registry client and TTL cache
- `rust/tcl-pkg/src/venv.rs` — virtual environment management
- `rust/tcl-pkg/src/docker.rs` — Dockerfile generation for Tcl projects
- `rust/tcl-cli/src/commands/docker.rs` — `tcl docker`'s verb handler, which
  lists the native extensions from the project's packs
- `rust/tcl-cli/src/commands/pkg_discover.rs` — `tcl pkg discover`
- `rust/tcl-pkg/src/exec.rs` — the single external-execution chokepoint
- `rust/tcl-pkg/src/policy.rs`, `hooks.rs` — operator policy and lifecycle hooks
- `rust/tcl-pkg/src/ui.rs` — CLI output helpers
- `rust/tcl-pkg-model/src/errors.rs` — error types; the CLI prints `Display` verbatim
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

- [tclpkg security architecture](security.md) — the sandbox,
  operator hooks, and locked-down policy layered over this engine.
- [package-loading contract](../contracts/package-loading.md) — the
  pre-existing `pkgIndex.tcl` loading path.
- [xdg-config contract](../contracts/xdg-config.md) — the XDG config and cache
  paths.
- [tcl verb CLI feature](../../kcs/features/kcs-feature-tcl-verb-cli.md) — the
  `tcl` CLI contracts.

## Review fixes (26 November 2026, deep review)

- **C6:** new section *SpecTcl facts and the analysis*. `tcl docker`'s
  extension list and `tcl pkg discover` are consumer 14 of the consumer
  migration (319), and the lockfile becomes a `tcl-db` input, so a lockfile
  edit re-tiers the packs. Anchors added for the two verb handlers.
