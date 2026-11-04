# Tcl startup `argc` can be unset and rebound

## Question

Does the startup `argc` binding remain immutable, or can an ordinary script remove it and install a new value? This is reference runtime evidence for issue #2243. It does not show that a compiler recognises a later tainted write.

## Assumptions

Each row used a fresh process on 2026-11-04 UTC. The exact script below was supplied on standard input with no command-line arguments; `LC_ALL=C`, working directory `/workspace/tcl-lsp/tmp/lsp-providers`. Other environment details: Not recorded. No Tcl package was sourced by the script.

## Script

```tcl-visible
puts [list VERSION [info patchlevel]]
puts [list BEFORE [info exists argc] $argc]
unset argc
puts [list AFTER_UNSET [info exists argc]]
set argc 7
puts [list AFTER_SET [info exists argc] $argc]
```

Script SHA-256: `353440684962a710ec2467fa417699b9bf0afb8c384ef953cb5eac2fd3aa87b1`.

## Results

### Tcl 8.4.20

- Launch argv: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.4`
- Input channel: standard input.
- Command-line arguments: none.
- Exit status: 0
- Standard output:

```text-visible
VERSION 8.4.20
BEFORE 1 0
AFTER_UNSET 0
AFTER_SET 1 7
```

- Standard error: empty.

### Tcl 8.5.19

- Launch argv: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.5`
- Input channel: standard input.
- Command-line arguments: none.
- Exit status: 0
- Standard output:

```text-visible
VERSION 8.5.19
BEFORE 1 0
AFTER_UNSET 0
AFTER_SET 1 7
```

- Standard error: empty.

### Tcl 8.6.18

- Launch argv: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.6`
- Input channel: standard input.
- Command-line arguments: none.
- Exit status: 0
- Standard output:

```text-visible
VERSION 8.6.18
BEFORE 1 0
AFTER_UNSET 0
AFTER_SET 1 7
```

- Standard error: empty.

### Tcl 9.0.4

- Launch argv: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh9.0`
- Input channel: standard input.
- Command-line arguments: none.
- Exit status: 0
- Standard output:

```text-visible
VERSION 9.0.4
BEFORE 1 0
AFTER_UNSET 0
AFTER_SET 1 7
```

- Standard error: empty.

### Tcl 9.1.0

- Launch argv: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh9.1`
- Input channel: standard input.
- Command-line arguments: none.
- Exit status: 0
- Standard output:

```text-visible
VERSION 9.1.0
BEFORE 1 0
AFTER_UNSET 0
AFTER_SET 1 7
```

- Standard error: empty.

### JimTcl

- Launch argv: `/workspace/tcl-lsp/tmp/lsp-oracles/jimtcl/jimsh`
- Input channel: standard input.
- Command-line arguments: none.
- Exit status: 0
- Standard output:

```text-visible
VERSION 0.84-10-g82fb4d0
BEFORE 1 0
AFTER_UNSET 0
AFTER_SET 1 7
```

- Standard error: empty.

## Observation

In each recorded interpreter, `argc` starts bound to `0`, is absent after `unset`, and is bound to `7` after `set`. Startup metadata therefore cannot by itself certify a later read as clean. This capture does not establish taint parity, SSA versions, or behaviour under another launch channel.
