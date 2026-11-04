# Provided packages and loader mutation

## Question

Does requiring an already provided package invoke a registered loader, including when the requested version is incompatible, and can an external package or a forgotten core package execute a loader that creates commands? These controls bound any proposed preservation of source command-table knowledge after `package require`; they do not establish a compiler repair.

## Assumptions

Each row used a fresh process on 2026-11-04 UTC. The exact script below was supplied on standard input with no command-line arguments. `LC_ALL=C`; current working directory was `/workspace/tcl-lsp/tmp/lsp-providers`. Other inherited environment details: Not recorded. The script supplies its own package names and loader bodies. The incompatible requirement is caught. The core-package reload is attempted only after this script explicitly forgets the native `Tcl` provision and registers an `ifneeded` script. The external and forgotten-core cases are not executed when `package ifneeded` is unavailable, as recorded by the script. Jim is the exact checkout `82fb4d0caf90240d94352a31fcbd16adf52658a7` and reports `0.84-10-g82fb4d0`.

## Script

```tcl-visible
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
puts [list VERSION [info patchlevel]]
set loader_hits 0
package provide __lsp2243_provided 1.2
set registration_code [catch {
    package ifneeded __lsp2243_provided 1.2 {
        incr loader_hits
        proc __lsp2243_unexpected {} {return changed}
        package provide __lsp2243_provided 1.2
    }
} registration_result]
puts [list IFNEEDED_REGISTRATION_CODE $registration_code RESULT $registration_result]
set provided_code [catch {package require __lsp2243_provided 1.0} provided_result]
puts [list PROVIDED_REQUIRE_CODE $provided_code RESULT $provided_result LOADER_HITS $loader_hits]
set incompatible_code [catch {package require __lsp2243_provided 2.0} incompatible_result]
puts [list INCOMPATIBLE_PROVIDED_CODE $incompatible_code RESULT $incompatible_result LOADER_HITS $loader_hits]
if {$registration_code == 0} {
    package ifneeded __lsp2243_external 1.2 {
        incr loader_hits
        proc __lsp2243_loaded {} {return changed}
        package provide __lsp2243_external 1.2
    }
    set external_code [catch {package require __lsp2243_external 1.0} external_result]
    puts [list EXTERNAL_REQUIRE_CODE $external_code RESULT $external_result LOADER_HITS $loader_hits COMMAND_EXISTS [llength [info commands __lsp2243_loaded]]]
    set core_version [package provide Tcl]
    package forget Tcl
    package ifneeded Tcl $core_version {
        incr loader_hits
        proc __lsp2243_core_reloaded {} {return changed}
        package provide Tcl $core_version
    }
    set forgotten_code [catch {package require Tcl $core_version} forgotten_result]
    puts [list FORGOTTEN_CORE_REQUIRE_CODE $forgotten_code RESULT $forgotten_result LOADER_HITS $loader_hits COMMAND_EXISTS [llength [info commands __lsp2243_core_reloaded]]]
} else {
    puts {EXTERNAL_AND_FORGOTTEN_CASES not-executed-ifneeded-unavailable}
}
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
IFNEEDED_REGISTRATION_CODE 0 RESULT {}
PROVIDED_REQUIRE_CODE 0 RESULT 1.2 LOADER_HITS 0
INCOMPATIBLE_PROVIDED_CODE 1 RESULT {version conflict for package "__lsp2243_provided": have 1.2, need 2.0} LOADER_HITS 0
EXTERNAL_REQUIRE_CODE 0 RESULT 1.2 LOADER_HITS 1 COMMAND_EXISTS 1
FORGOTTEN_CORE_REQUIRE_CODE 0 RESULT 8.4 LOADER_HITS 2 COMMAND_EXISTS 1
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
IFNEEDED_REGISTRATION_CODE 0 RESULT {}
PROVIDED_REQUIRE_CODE 0 RESULT 1.2 LOADER_HITS 0
INCOMPATIBLE_PROVIDED_CODE 1 RESULT {version conflict for package "__lsp2243_provided": have 1.2, need 2.0} LOADER_HITS 0
EXTERNAL_REQUIRE_CODE 0 RESULT 1.2 LOADER_HITS 1 COMMAND_EXISTS 1
FORGOTTEN_CORE_REQUIRE_CODE 0 RESULT 8.5.19 LOADER_HITS 2 COMMAND_EXISTS 1
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
IFNEEDED_REGISTRATION_CODE 0 RESULT {}
PROVIDED_REQUIRE_CODE 0 RESULT 1.2 LOADER_HITS 0
INCOMPATIBLE_PROVIDED_CODE 1 RESULT {version conflict for package "__lsp2243_provided": have 1.2, need 2.0} LOADER_HITS 0
EXTERNAL_REQUIRE_CODE 0 RESULT 1.2 LOADER_HITS 1 COMMAND_EXISTS 1
FORGOTTEN_CORE_REQUIRE_CODE 0 RESULT 8.6.18 LOADER_HITS 2 COMMAND_EXISTS 1
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
IFNEEDED_REGISTRATION_CODE 0 RESULT {}
PROVIDED_REQUIRE_CODE 0 RESULT 1.2 LOADER_HITS 0
INCOMPATIBLE_PROVIDED_CODE 1 RESULT {version conflict for package "__lsp2243_provided": have 1.2, need 2.0} LOADER_HITS 0
EXTERNAL_REQUIRE_CODE 0 RESULT 1.2 LOADER_HITS 1 COMMAND_EXISTS 1
FORGOTTEN_CORE_REQUIRE_CODE 0 RESULT 9.0.4 LOADER_HITS 2 COMMAND_EXISTS 1
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
IFNEEDED_REGISTRATION_CODE 0 RESULT {}
PROVIDED_REQUIRE_CODE 0 RESULT 1.2 LOADER_HITS 0
INCOMPATIBLE_PROVIDED_CODE 1 RESULT {version conflict for package "__lsp2243_provided": have 1.2, need 2.0} LOADER_HITS 0
EXTERNAL_REQUIRE_CODE 0 RESULT 1.2 LOADER_HITS 1 COMMAND_EXISTS 1
FORGOTTEN_CORE_REQUIRE_CODE 0 RESULT 9.1.0 LOADER_HITS 2 COMMAND_EXISTS 1
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
IFNEEDED_REGISTRATION_CODE 1 RESULT {package, unknown command "ifneeded": should be forget, names, provide, require}
PROVIDED_REQUIRE_CODE 0 RESULT 1.0 LOADER_HITS 0
INCOMPATIBLE_PROVIDED_CODE 0 RESULT 1.0 LOADER_HITS 0
EXTERNAL_AND_FORGOTTEN_CASES not-executed-ifneeded-unavailable
```

- Standard error: empty.

## Observation

On all five recorded C Tcl releases, the compatible already-provided request runs no loader; the incompatible request errors and still runs no loader. The external request and forgotten-core request each execute their loader and create a command. Jim rejects `ifneeded`, so those two cases are unrun there; its observed provided-package version results differ from C Tcl and cannot borrow the C rule.
