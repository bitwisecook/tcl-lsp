# Tcl startup `argc` in stdin-launched interpreters

## Question

Is `argc` bound before the supplied script reads it when an interpreter receives the script on standard input with no command-line arguments? This is reference startup evidence for issue #2243; it does not test memoised compiler diagnostics.

## Assumptions

Each row used a fresh process on 2026-11-04 UTC. The process received the exact script below on standard input, with no command-line arguments. `LC_ALL=C`; current working directory was `/workspace/tcl-lsp/tmp/lsp-providers`. Other environment details: Not recorded. No Tcl package was sourced by the script.

## Script

```tcl-visible
puts [list VERSION [info patchlevel]]
puts [list ARGC_EXISTS [info exists argc] ARGC_VALUE $argc]
```

Script SHA-256: `7de014d6c0c799b346fde26f4363f792ab1662c145af5d3d27e01fee4e138bb8`.

## Results

### Tcl 8.4.20

- Launch argv: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.4`
- Input channel: standard input.
- Command-line arguments: none.
- Exit status: 0
- Standard output:

```text-visible
VERSION 8.4.20
ARGC_EXISTS 1 ARGC_VALUE 0
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
ARGC_EXISTS 1 ARGC_VALUE 0
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
ARGC_EXISTS 1 ARGC_VALUE 0
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
ARGC_EXISTS 1 ARGC_VALUE 0
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
ARGC_EXISTS 1 ARGC_VALUE 0
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
ARGC_EXISTS 1 ARGC_VALUE 0
```

- Standard error: empty.

## Observation

Each recorded interpreter reports `argc` as present with value `0` in this launch channel. This observation does not establish the compiler’s SSA version, taint provenance, cache parity, or behaviour under a different launch channel.
