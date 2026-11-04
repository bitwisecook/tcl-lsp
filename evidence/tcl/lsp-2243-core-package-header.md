# Core-package requirement in the IMAP4 header

## Question

Does the exact Tcllib 2.0 IMAP4 header command `package require Tcl 8.5 9` complete in fresh C Tcl and Jim processes, and does the measured set of global command names change? The distinction matters to issue #2243 because source command-table knowledge becomes unavailable immediately after this invocation. This run does not test compiler source coverage, native command allocation identity, or the entire IMAP4 file.

## Assumptions

Each row used a fresh process on 2026-11-04 UTC. The exact script below was supplied on standard input with no command-line arguments. `LC_ALL=C`; current working directory was `/workspace/tcl-lsp/tmp/lsp-providers`. Other inherited environment details: Not recorded. The header command is caught so that its error and the following `package provide imap4 0.5.5` can be recorded even on a release that rejects the header. Global command-name equality is the measured observation; it is not an allocation or implementation-identity receipt. The script installs no loaders or execution traces.

## Script

```tcl-visible
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
puts [list VERSION [info patchlevel]]
set provision_code [catch {package provide Tcl} provided]
puts [list INITIAL_TCL_PROVISION_CODE $provision_code VALUE $provided]
set commands_before [lsort [info commands ::*]]
set header_code [catch {package require Tcl 8.5 9} header_result]
set commands_after [lsort [info commands ::*]]
puts [list EXACT_HEADER_CODE $header_code RESULT $header_result]
puts [list GLOBAL_COMMAND_NAMES_UNCHANGED [string equal $commands_before $commands_after]]
set provide_code [catch {package provide imap4 0.5.5} provide_result]
puts [list IMAP4_PROVIDE_CODE $provide_code RESULT $provide_result]
```

## Results

### Tcl 8.4.20

- Launch argv: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.4`
- Input channel: standard input.
- Command-line arguments: none.
- Exit status: 0
- Standard output:

```text-visible
VERSION 8.4.20
INITIAL_TCL_PROVISION_CODE 0 VALUE 8.4
EXACT_HEADER_CODE 1 RESULT {wrong # args: should be "package require ?-exact? package ?version?"}
GLOBAL_COMMAND_NAMES_UNCHANGED 1
IMAP4_PROVIDE_CODE 0 RESULT {}
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
INITIAL_TCL_PROVISION_CODE 0 VALUE 8.5.19
EXACT_HEADER_CODE 0 RESULT 8.5.19
GLOBAL_COMMAND_NAMES_UNCHANGED 1
IMAP4_PROVIDE_CODE 0 RESULT {}
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
INITIAL_TCL_PROVISION_CODE 0 VALUE 8.6.18
EXACT_HEADER_CODE 0 RESULT 8.6.18
GLOBAL_COMMAND_NAMES_UNCHANGED 1
IMAP4_PROVIDE_CODE 0 RESULT {}
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
INITIAL_TCL_PROVISION_CODE 0 VALUE 9.0.4
EXACT_HEADER_CODE 0 RESULT 9.0.4
GLOBAL_COMMAND_NAMES_UNCHANGED 1
IMAP4_PROVIDE_CODE 0 RESULT {}
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
INITIAL_TCL_PROVISION_CODE 0 VALUE 9.1.0
EXACT_HEADER_CODE 0 RESULT 9.1.0
GLOBAL_COMMAND_NAMES_UNCHANGED 1
IMAP4_PROVIDE_CODE 0 RESULT {}
```

- Standard error: empty.

### Jim 0.84-10-g82fb4d0

- Launch argv: `/workspace/tcl-lsp/tmp/lsp-oracles/jimtcl/jimsh`
- Input channel: standard input.
- Command-line arguments: none.
- Exit status: 0
- Standard output:

```text-visible
VERSION 0.84-10-g82fb4d0
INITIAL_TCL_PROVISION_CODE 0 VALUE {}
EXACT_HEADER_CODE 1 RESULT {wrong # args: should be "package require name ?version?"}
GLOBAL_COMMAND_NAMES_UNCHANGED 1
IMAP4_PROVIDE_CODE 0 RESULT {}
```

- Standard error: empty.

## Observation

The exact header succeeds on C Tcl 8.5–9.1 and is rejected for argument count on Tcl 8.4 and this Jim build. The global command-name snapshot is unchanged in every measured case. The later package-provision command succeeds in each caught experiment; this does not mean the original uncaught file continues after a header error.
