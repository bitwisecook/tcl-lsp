# KCS: feature — Unknown Command Resolution (W123)

> **Audience:** User
> **Type:** Functionality

## Summary

Detects unresolved commands and offers "did you mean?" suggestions. Static
analysis of user-defined `unknown` procs extracts dispatch targets to reduce
false positives.

## Applies to

all-editors, jetbrains (other editors via XDG config), warning

## How to use

- **Enable**: W123 is on by default. Set `tclLsp.diagnostics.W123 = false` in your editor settings to disable it.
- **Diagnostics**: Unresolved commands appear as HINT-level underlines. Hover shows the message and any "did you mean?" suggestion.
- **Code actions**: When a suggestion is available, a quick-fix replaces the command name.
- **Suppress per-line**: Append `# noqa: W123` to suppress on a specific line.

### Recognised `unknown` proc patterns

When the analyser encounters `proc unknown {cmd args} { ... }` (or the
qualified form `proc ::tcl::unknown ...`), it inspects the body and adjusts
W123 behaviour accordingly:

| Pattern | Example | Effect |
|---------|---------|--------|
| **switch -exact dispatch** | `switch $cmd { foo {...} bar {...} }` | `foo`, `bar` are treated as known commands |
| **switch -glob/-regexp** | `switch -glob $cmd { fo* {...} }` | Conservative: W123 suppressed entirely |
| **Empty stub** | `proc unknown {args} {}` | Nothing resolves; W123 fires for all unknown commands |
| **Chain to original** | `_original_unknown $cmd {*}$args` | Conservative: W123 suppressed entirely |
| **auto\_load** | `auto_load $cmd` | Conservative: W123 suppressed entirely |
| **exec** | `exec $cmd {*}$args` | Conservative: W123 suppressed entirely |
| **Case-insensitive** | `switch [string tolower $cmd] {...}` | All known commands match; W123 suppressed |

### Other suppression mechanisms

- **Dynamic providers**: If `load`, `set auto_path`, `lappend auto_path`, `rename`, or `namespace import` is detected, W123 is suppressed for the entire file.
- **Package require**: If any `package require` is present, W123 is suppressed (external packages may define commands).
- **Dialect stubs**: Commands declared via `# tcl-lsp: stub` are treated as known. See [Dialect Stubs](../../../docs/design/contracts/dialect-stubs.md).
- **User-defined procs**: Any `proc` defined in the file (or sourced via packages) is a known command.
- **Command aliases**: Commands defined via `interp alias` are treated as known. See [Command Alias Resolution](../../../docs/design/contracts/command-alias-resolution.md).
- **Namespace-qualified names**: Commands containing `::` are skipped (may come from `namespace import`).
- **Library and package commands**: A command an installed library auto-loads
  (through a `tclIndex` on the configured library paths), or that a package the
  document requires defines, is treated as known — see below.

### Guarded packages and the Tcl version you target

A package's `pkgIndex.tcl` may only register itself on some Tcl releases. The
tcllib idiom guards the whole file:

```tcl
if {![package vsatisfies [package provide Tcl] 8.5 9]} {return}
package ifneeded log 1.5 [list source [file join $dir log.tcl]]
```

and the idiom C extensions use picks a branch:

```tcl
if {[package vsatisfies [package provide Tcl] 9.0-]} {
    package ifneeded mypkg 1.0 [list load [file join $dir mypkg9.so]]
} else {
    package ifneeded mypkg 1.0 [list load [file join $dir mypkg8.so]]
}
```

The server reads these guards and evaluates them against the Tcl version the
document targets (the `tclLsp.dialect` setting, a folder override, or the
version detected from the file). The outcome decides whether W123 is
suppressed:

| The guard says | W123 |
|---|---|
| the package **does** register on your Tcl version | suppressed — the command is real |
| the guard cannot be read (it tests the platform, a file, or a variable) | suppressed — the server does not guess |
| the package **does not** register on your Tcl version | **shown** — `package require` would fail here, so the call really is an error |

The last row is the case that used to be missed: a package that cannot load on
Tcl 9 no longer silences the warning for a Tcl 9 workspace. If you see W123 on
a command you believe exists, check the `tclLsp.dialect` setting first.

## Operational context

W123 runs as a **post-analysis pass** after all proc definitions have been
collected.  This means forward-defined procs and `unknown` handlers defined
later in the file are still captured.

The "did you mean?" engine uses Levenshtein edit distance (max distance 2)
against the union of: registry commands, user-defined procs, stub commands,
`unknown` dispatch targets, and command alias names.

## File-path anchors

- `rust/tcl-compiler/src/analyser/diagnostics/unresolved.rs` — the W123 pass and
  its known-name sets
- `rust/tcl-lsp-server/src/lib.rs` — `refine_w123_diagnostics`, the workspace
  refinement that consults the package database
- `rust/tcl-lsp-core/src/package_resolver.rs` — the `auto_path` / `pkgIndex` /
  `tclIndex` package database
- `rust/tcl-lsp-core/src/package_resolver/reachability.rs` — which
  `package ifneeded` declarations a given Tcl release actually runs

## Failure modes

- False positives in codebases using dynamic command creation (e.g. `apply`, `coroutine`) not detected by the gating logic.
- Forward-defined `unknown` proc in a sourced file (cross-file) is not detected — analysis is single-file.
- A `pkgIndex.tcl` guard the server cannot read leaves the package "possibly
  available", so a command it would never really provide is not flagged. That
  is deliberate: a missed hint is a smaller problem than a warning on working
  code.

## Test anchors

- `rust/tcl-lsp-core/src/package_resolver/reachability/tests.rs` — guard
  evaluation, per Tcl release
- `rust/tcl-lsp-server/tests/e2e/diagnostics.rs` — end-to-end W123 cases,
  including the guarded-package pair

## Example

With `tclLsp.diagnostics.W123` at its default (enabled):

```tcl
proc greet {name} {
    puts "Hello, $name"
}

gret "Alice"
```

Line 5 shows a hint-level squiggle under `gret` with the message
`W123 unresolved command 'gret' — did you mean 'greet'?`. A
lightbulb code action offers **Replace with `greet`** which
rewrites the call in one click.

## Discoverability

- [KCS feature index](README.md)
- [Diagnostics calculation](../../../docs/design/compiler/diagnostics-calculation.md)
- [Dialect stubs](../../../docs/design/contracts/dialect-stubs.md)
