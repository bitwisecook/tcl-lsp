# Packages: one correct owner, three others beside it

How `package require` and friends, package versions, `pkgIndex.tcl`/`auto_path` discovery and Tcl modules are handled across execution (VM, runtime) and analysis and tooling (compiler, analyser, LSP, server, package manager, SpecTcl). Part of [tidying the architecture](architecture-tidy-plan.md).

**Verdict:** execution is in good shape: both engines use one Tcl-correct version owner (`tcl-dialect/src/version/package_versions.rs`, a port of `tclPkg.c`) and the real Tcl library scripts for discovery. Analysis and tooling run three more version schemes, match package names byte for byte with no alias model, cannot see Tcl modules, search in a different order from Tcl, and have no single list of which packages ship with which release.

## Measured behaviour

The fixtures in [`oo-dispatch-probe/packages/`](handover/oo-dispatch-probe/packages/) cover `vcompare` and `vsatisfies` edge cases (alpha/beta, `1.2` versus `1.10`, leading zeros, ranges including `min-`, reversed and 9.0 forms), invalid versions, `ifneeded`/`require`/`present`, `-exact`, `prefer stable|latest`, `package unknown`, `forget`, multiple requirements, a provide/require mismatch, case-sensitive names, `pkgIndex.tcl` through `auto_path`, `.tm` modules through `::tcl::tm::path`, and bundled packages. All agree across tclsh 8.6.18, 9.0.4 and 9.1.0; 8.4.20 rejects the 8.5 grammar (alpha/beta, ranges, multiple requirements, modules) and accepts a mismatched `package provide`, as expected.

**Execution.** All 24 runs of the version and selection fixtures (8.6 and 9.0) match tclsh in both the VM and the runtime. Discovery needs the Tcl library: with `TCL_LIBRARY` pointing at each release's `library`, the VM finds `pkgIndex.tcl` packages and `.tm` modules exactly as tclsh does. Two faults:

- **The VM exits silently on `msgcat`.** `package require msgcat` (Tcl 9.0.4 library), and even `catch {source $tcl_library/msgcat/msgcat.tcl}`, end the process with status 1 and no output; the error never reaches the script. `tcltest` loads correctly.
- **The runtime's script runner has no standard library** on the host: no `auto_path`, no `package unknown`, no `::tcl::tm`, even with `TCL_LIBRARY` set. The runtime embeds the library for WASM; whether its native entry is meant to bootstrap it is unclear from the runner.
- Minor: the VM's `auto_path` ends with a relative `./lib` where tclsh has an absolute installation path.

**Analysis.** `package require itcl` followed by `itcl::class` draws W120 "requires `package require Itcl`"; itcl 4 provides both names (its own `pkgIndex.tcl`, measured earlier with itcl 4.3.5).

## Version implementations

| Implementation | Grammar | Used by |
| --- | --- | --- |
| `tcl-dialect` `package_versions.rs` | Tcl's, per release (8.4 without ranges or alpha/beta; 9.x `+suffix`); oracle-tested against 8.6.14 and 9.0.4 (2,627 rows) | both engines, compiler trusted-loader selection, LSP pkgIndex reachability, Tk preview, Spec Studio, `VersionSet` |
| `tcl-registry` `version.rs` | dotted numbers only; **prerelease-blind by design** (`1.0a1` = `1.0b1` = `1.0`, *read*); no release grammar; an empty requirement list satisfies | the version gate diagnostics, the LSP document floor, SpecTcl pack versions, `tcl pkg discover`, MCP |
| `tcl-pkg-model` `version.rs` | semver (a `v` prefix, `rc`, `+build`; at most three numeric parts, no leading zeros) | manifests, the package manager's resolver, the CLI |
| `tcl-sslictcl` `trust.rs` | digit runs, falling back to string order | SslicTcl trust |

`tcl pkg discover` turns a Tcl requirement into a manifest minimum through the registry comparator and then semver: a bare `1.2` (Tcl: at least 1.2, below 2) becomes `>=1.2`, so the resolver may choose a 2.x the script's own `package require` refuses (inferred). No oracle checks the second, third or fourth row.

## Availability and names

Commands carry `required_package`; W120 (`analyser/diagnostics/unresolved.rs`) compares the written package name with it byte for byte (`matches_ascii`, *read*), with no case folding, no alias table and no version. The compiler's package loader matching is case-sensitive too. The server's refinement (`tcl-lsp-server/src/diagnostics/package_refinement.rs`) marks availability "unknowable" when a required package is neither registry-provided nor in its scanned index, and then drops every W120 in that scope, which can hide the false positive and every other W120 with it (inferred).

## Discovery

- **Engines:** `auto_path` from the shared `tcl_platform::bootstrap::auto_path` (`TCLLIBPATH` first), then `tclPkgUnknown` and the module handler from the real `package.tcl` and `tm.tcl`.
- **LSP:** its own `PackageResolver` scans `pkgIndex.tcl` and `tclIndex`, **with no `.tm` support anywhere** in the compiler, LSP, server, package manager or CLI; its search order puts workspace trees first and `TCLLIBPATH` last (`tcl-lsp-server/src/workspace/package_database.rs`, *read*), merges every discovered installation whatever the document's dialect, and decodes `TCLLIBPATH` with different rules from the package manager's `venv.rs`.
- **Analysis:** `auto_path_eval.rs` evaluates a document's own `lappend auto_path` and feeds the server.

## Models and rosters

No package model is shared: the VM's package table, the runtime's `PackageState`, the LSP's `PackageInfo`/`PackageResolver`, the compiler's `TrustedPackageLoader` and the package manager's manifest and lockfile types are all separate. `tcl-lsp.listKnownPackages` always returns an empty list, with a comment that is now stale.

Which packages ship with which release is spread over `core_provided_packages` (Tcl, TclOO only), two copies of the Itcl and Tk pins (`LibraryPin` and `PackagePlacement`; Itcl 3.4 for 8.4/8.5 is doubtful, and 9.0 ships Itcl 4.3), the runtime's embedded library (one release's), the tcllib table and per-command `required_package`. Thread, tdbc, sqlite3, `tcl::idna` and `http` are in none of them, although Tcl's own `pkgs/package.list.txt` lists the bundled set for each source release.

## Conformance

The version oracle (8.6.14, 9.0.4) drives only the `tcl-dialect` owner; `package_lookup_vectors.txt` (13 rows) drives the engines and compiler; upstream `package.test` slices run on the VM. Nothing runs `tm.test` or `pkgMkIndex.test` slices, and nothing checks the LSP search order, W120 name matching or the other version schemes against an interpreter.

## Plan

1. **Fix the engine faults:** the VM's silent exit while loading `msgcat` (find the construct, report it as an error, add the library packages to the engine suites); decide whether the runtime's native entry bootstraps the standard library and make its runner do so; use an absolute library path in the VM's `auto_path`.
2. **One version owner:** route the version gate, the LSP document floor, SpecTcl pack versions, `tcl pkg discover`, MCP and SslicTcl through `tcl-dialect`'s implementation; delete the registry comparator's version logic; extend the oracle to 8.4 and 9.1.
3. **Manifest requirements in Tcl terms:** let the package manager hold Tcl requirements (ranges, the implicit major cap, alpha/beta) or translate them exactly; keep semver only where an external ecosystem demands it.
4. **Package identity with aliases:** a package-name owner (with the registry declaring that `itcl` and `Itcl` are one package, and similar cases) used by W120, the compiler's loaders and the server; include the required version; remove the "unknowable drops every W120" rule in favour of per-package unknowns.
5. **One discovery model** shared by the engines' semantics and the LSP: Tcl's search order (`TCLLIBPATH` first), per-dialect installations, `pkgIndex.tcl`, `tclIndex` and **Tcl modules** with their naming rules; one `TCLLIBPATH` decoder.
6. **One bundled-package roster** per release, generated from each Tcl source tree's `pkgs/package.list.txt` and library, read by the dialect profiles, the registry, the engines and the LSP; correct the Itcl pins.
7. **Conformance:** the version oracle for every implementation that remains, `tm.test` and `pkgMkIndex.test` slices on both engines, and fixtures for LSP search order and W120 name matching against tclsh.
