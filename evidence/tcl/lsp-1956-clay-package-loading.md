# Clay package loading with exact module paths

## Question

With exact known Tcllib 2.0 Clay and MD5 module paths, which recorded shells load Clay, and is TclOO actually installed in these environments? This follow-up avoids the earlier probe's unsupported Jim glob option. It tests package loading and existing commands, not compiler availability advice, receiver entry or all possible installations of a Tcl release.

## Assumptions

Each row used a fresh process on 2026-11-04 UTC with the exact script below on standard input and no command-line arguments. `LC_ALL=C`; current working directory was `/workspace/tcl-lsp/tmp/lsp-providers`. Other inherited environment details: Not recorded. Only the two named original module directories are appended to `auto_path`; no substitute package or command is installed by the probe. Package initialization may load actual dependencies. A failure to find TclOO here says nothing about another host with a TclOO extension installed. The Clay file SHA-256 is `150b48a3ddf4916cca5e182ce05274773ddcc3bc296c9b1e6f8c4d9afd1c49b0`.

## Script

```tcl-visible
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
puts [list VERSION [info patchlevel]]
puts [list INITIAL_OO_CLASS [info commands ::oo::class]]
set oo_code [catch {package require TclOO} oo_result]
puts [list TCLOO_PACKAGE_CODE $oo_code RESULT $oo_result]
lappend auto_path /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/clay
lappend auto_path /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/md5
set clay_code [catch {package require clay 0.7} clay_result]
puts [list CLAY_CODE $clay_code RESULT $clay_result]
puts [list FINAL_OO_CLASS [info commands ::oo::class]]
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
INITIAL_OO_CLASS {}
TCLOO_PACKAGE_CODE 1 RESULT {can't find package TclOO}
CLAY_CODE 1 RESULT {can't find package clay 0.7}
FINAL_OO_CLASS {}
```

- Standard error:

```text-visible
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/md5/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/clay/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
```

### Tcl 8.5.19

- Launch argv: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.5`
- Input channel: standard input.
- Command-line arguments: none.
- Exit status: 0
- Standard output:

```text-visible
VERSION 8.5.19
INITIAL_OO_CLASS {}
TCLOO_PACKAGE_CODE 1 RESULT {can't find package TclOO}
CLAY_CODE 1 RESULT {can't find package clay 0.7}
FINAL_OO_CLASS {}
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
INITIAL_OO_CLASS ::oo::class
TCLOO_PACKAGE_CODE 0 RESULT 1.1.0
CLAY_CODE 0 RESULT 0.8.8
FINAL_OO_CLASS ::oo::class
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
INITIAL_OO_CLASS ::oo::class
TCLOO_PACKAGE_CODE 0 RESULT 1.3.1
CLAY_CODE 0 RESULT 0.8.8
FINAL_OO_CLASS ::oo::class
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
INITIAL_OO_CLASS ::oo::class
TCLOO_PACKAGE_CODE 0 RESULT 1.3.1
CLAY_CODE 0 RESULT 0.8.8
FINAL_OO_CLASS ::oo::class
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
INITIAL_OO_CLASS {}
TCLOO_PACKAGE_CODE 1 RESULT {Can't load package TclOO}
CLAY_CODE 1 RESULT {wrong # args: should be "package require name ?version?"
Can't load package clay}
FINAL_OO_CLASS {}
```

- Standard error: empty.

## Observation

C Tcl 8.6.18, 9.0.4 and 9.1.0 load Clay 0.8.8; TclOO reports versions 1.1.0, 1.3.1 and 1.3.1. These Tcl 8.4 and 8.5 installations have neither `::oo::class` nor a loadable TclOO package and cannot find this pinned Clay release. The Jim process reaches Clay package loading and records its multi-requirement `package require Tcl 8.6 9` argument-count error, followed by failure to load Clay. It also has no `::oo::class` or loadable TclOO package. No result is inferred for an unrecorded extension or modified bootstrap.
