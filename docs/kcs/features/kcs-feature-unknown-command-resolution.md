# KCS: feature — Unknown Command Resolution (W123)

> **Audience:** User
> **Type:** Functionality

## Summary

Detects unresolved commands and offers "did you mean?" suggestions. Static
analysis of user-defined `unknown` procs extracts dispatch targets to reduce
false positives.

## Applies to

all-editors, warning

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

### Positioned command presence and remaining abstentions

W123 asks whether the selected command slot is absent at this invocation's
lookup point. The shared `SourceCommandBindings` owner tracks supported
source command transitions at their positions and the diagnostic-purpose
presence query returns `Present`, `Absent`, `MayPresent`, or `Unknown`. W123
is emitted only for `Absent`; it reports an unresolved selected slot, not
that execution must fail. A present alias/import wrapper counts as a slot
even when its target is missing, and an absent slot may still be served by
autoload or another fallback.

Dynamic or unsupported providers do not suppress W123 for the whole file.
They make the affected lookup world unknown when the owner cannot retain a
complete applicable binding. Qualified names are not skipped categorically;
the selected namespace policy, retained qualifier, and fallback state must
support an `Absent` result or W123 abstains.

- **Stubs and explicit assistance declarations** can suppress this diagnostic
  at covered sites, but they do not establish a runtime callable or registry
  implementation. See [Dialect Stubs](../../../docs/design/contracts/dialect-stubs.md).
- **Source declarations and aliases** count only when the positioned owner
  proves that they occupy the selected slot at that call. A declaration
  elsewhere in the file is not blanket proof. See [Command Alias Resolution](../../../docs/design/contracts/command-alias-resolution.md).
- **Package and library providers** contribute only where retained workspace
  and source evidence supports the lookup; a `package require` statement alone
  does not suppress W123 across the file.

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

If you see W123 on a command you believe exists, check the `tclLsp.dialect`
setting first: a package that cannot load on the Tcl version you target does
not silence the warning.

## Operational context

W123 runs as a **post-analysis pass** after all proc definitions have been
collected.  This means forward-defined procs and `unknown` handlers defined
later in the file are still captured.

The "did you mean?" engine uses edit distance against the union of registry
commands, user-defined procs, stub commands, `unknown` dispatch targets, and
command alias names. The budget scales with the name's length — one edit for a
short name, up to three for a long one — so a long name is never rewritten
wholesale.

## Failure modes

- False positives in codebases using dynamic command creation (e.g. `apply`, `coroutine`) not detected by the gating logic.
- Forward-defined `unknown` proc in a sourced file (cross-file) is not detected — analysis is single-file.
- A `pkgIndex.tcl` guard the server cannot read leaves the package "possibly
  available", so a command it would never really provide is not flagged. That
  is deliberate: a missed hint is a smaller problem than a warning on working
  code.

## Example

With `tclLsp.diagnostics.W123` at its default (enabled):

```tcl
proc greet {name} {
    puts "Hello, $name"
}

gret "Alice"
```

Line 5 shows a hint-level squiggle under `gret` with the message
`W123 Unknown command 'gret'; did you mean 'greet'?`. A lightbulb
code action offers **Replace with 'greet'**, which rewrites the call
in one click.

## Discoverability

- [KCS feature index](README.md)
- [Diagnostics calculation](../../../docs/design/compiler/diagnostics-calculation.md)
- [Dialect stubs](../../../docs/design/contracts/dialect-stubs.md)
