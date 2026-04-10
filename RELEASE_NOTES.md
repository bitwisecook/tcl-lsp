# v1.7.0

## New Features
- **Conf-wrapped iRule dialect mode**: Full LSP support for `ltm rule` / `gtm rule` stanza files — diagnostics, symbols, semantic tokens, and formatting all work across embedded rule bodies with correct position mapping
- **Tcl package manager (`tclpkg`)**: New `tcl pkg` CLI with MVS-based dependency resolution, manifest files, lock support, `freeze` verb, and a pure-Tcl 8.6+ implementation alongside the Python one
- **Dockerfile generation**: `tcl docker` CLI verb generates production-ready Dockerfiles for Tcl projects with automatic dependency detection
- **W130–W134 diagnostics**: New warning codes for package management issues (missing manifests, unresolvable dependencies, version conflicts, stale lock files, unused dependencies)
- **Safe interpreter support**: `interp create -safe` now creates properly sandboxed child interpreters with empty command whitelists
- **VM opcodes**: `lset` command, `LSET_LIST`/`OVER` opcodes, `STR_CLASS` for `string is`, `TRY_CVT_TO_BOOLEAN`, and `chan` command family

## Improvements
- **VS Code formatting conventions**: Feature toggles now inherit from VS Code's `editor.*` globals by default instead of requiring custom settings, with tri-state (null/true/false) support
- **KCS documentation**: Comprehensive diagnostic pages (E-codes, W-codes, S-codes, T-codes, IRULE-codes), all 28 O-code optimisation pages, compiler-pass glossary, feature pages, and Applies-to tag vocabulary
- **Analyser accuracy**: Renamed `ArgRole.VAR_NAME` to `VAR_WRITE` with proper dual-shape resolver for `set`, fixing W210 false positives for `regexp`/`regsub` capture variable writes
- **Real tcltest integration**: `package require tcltest` now sources the genuine `tcltest.tcl` library when available, giving full test framework behaviour
- **External test suite runner** for pure-Tcl projects

## Bug Fixes
- Fix formatter corrupting `${variable}` and `{*}$variable` syntax
- Fix backslash substitution in interpolated strings
- Fix command substitution concatenation in bytecode compiler
- Fix `eof` command and `info script` in test runner
- Fix tcltest namespace import, `numTests` sync, `-output` handling, skip, and unknown guard
- Fix inlayHints resolver for string values
- Skip tcltest setup for safe interpreters to prevent `package` command errors

## Breaking Changes
- VS Code formatting settings (`tclLsp.format.*`) now default to `null` (inherit from editor globals) instead of explicit `true`/`false` — existing explicit settings are preserved
