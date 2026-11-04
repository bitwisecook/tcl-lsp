# Foreach numeric representation differs across Tcl releases

## Question

Does an ordinary foreach list consumer physically convert an integer object to a list on every supported Tcl release? This determines whether an integer input is a valid conversion-warning control for #940.

## Assumptions

Each row starts a fresh process, reads the exact script file below, and uses `/workspace/tcl-lsp` as its working directory. Environment variables and build flags: Not recorded. The process exit status and standard streams were captured separately. The representation observer is available only on Tcl 8.6, 9.0 and 9.1; older Tcl and Jim establish displayed results only. Before/after addresses are process-local observations of this script, not transferable object identity.

The [80-row native capture](../../experiments/shimmer-940/foreach-native-2026-11-04/README.md) also retains forced-handler, dynamic-length, double, bignum, dictionary, original-font and empty-list cases. Both native foreach routes preserve numeric representations on Tcl 9.0/9.1, while dictionaries convert to lists. No product diagnostic or release result is certified.

## Script

```tcl-visible
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
set values 0
incr values
set have_rep [expr {[llength [info commands ::tcl::unsupported::representation]] != 0}]
puts [list patchlevel [info patchlevel] have_rep $have_rep]
if {$have_rep} {puts [list before [::tcl::unsupported::representation $values]]}
set visited {}
foreach value $values {lappend visited $value}
if {$have_rep} {puts [list after [::tcl::unsupported::representation $values]]}
puts [list visited $visited]
```

## Results

### Tcl 8.4

- Recorded version: 8.4.20.
- Input channel: command-line script file.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.4 /tmp/lsp-940-foreach-route-oracles/integer-compiled.tcl`
- Working directory: `/workspace/tcl-lsp`
- Exit status: 0
- Standard output:

```text-visible
patchlevel 8.4.20 have_rep 0
visited 1
```

- Standard error:

```text-visible
⟦EOF⟧
```

### Tcl 8.5

- Recorded version: 8.5.19.
- Input channel: command-line script file.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.5 /tmp/lsp-940-foreach-route-oracles/integer-compiled.tcl`
- Working directory: `/workspace/tcl-lsp`
- Exit status: 0
- Standard output:

```text-visible
patchlevel 8.5.19 have_rep 0
visited 1
```

- Standard error:

```text-visible
⟦EOF⟧
```

### Tcl 8.6

- Recorded version: 8.6.18.
- Input channel: command-line script file.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.6 /tmp/lsp-940-foreach-route-oracles/integer-compiled.tcl`
- Working directory: `/workspace/tcl-lsp`
- Exit status: 0
- Standard output:

```text-visible
patchlevel 8.6.18 have_rep 1
before {value is a int with a refcount of 2, object pointer at 0x55cb5eb55920, internal representation 0x1:(nil), no string representation}
after {value is a list with a refcount of 2, object pointer at 0x55cb5eb55920, internal representation 0x55cb5eb67e10:(nil), string representation "1"}
visited 1
```

- Standard error:

```text-visible
⟦EOF⟧
```

### Tcl 9.0

- Recorded version: 9.0.4.
- Input channel: command-line script file.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh9.0 /tmp/lsp-940-foreach-route-oracles/integer-compiled.tcl`
- Working directory: `/workspace/tcl-lsp`
- Exit status: 0
- Standard output:

```text-visible
patchlevel 9.0.4 have_rep 1
before {value is a int with a refcount of 2, object pointer at 0x56328a0d7960, internal representation 0x1:0x56328a08ba10, no string representation}
after {value is a int with a refcount of 2, object pointer at 0x56328a0d7960, internal representation 0x1:0x56328a08ba10, no string representation}
visited 1
```

- Standard error:

```text-visible
⟦EOF⟧
```

### Tcl 9.1

- Recorded version: 9.1.0.
- Input channel: command-line script file.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh9.1 /tmp/lsp-940-foreach-route-oracles/integer-compiled.tcl`
- Working directory: `/workspace/tcl-lsp`
- Exit status: 0
- Standard output:

```text-visible
patchlevel 9.1.0 have_rep 1
before {value is a int with a refcount of 2, object pointer at 0x55d06b5e70f0, internal representation 0x1:0x55d06b59aa20, no string representation}
after {value is a int with a refcount of 2, object pointer at 0x55d06b5e70f0, internal representation 0x1:0x55d06b59aa20, no string representation}
visited 1
```

- Standard error:

```text-visible
⟦EOF⟧
```

### JimTcl

- Recorded version: 0.84-10-g82fb4d0.
- Input channel: command-line script file.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/jimtcl/jimsh /tmp/lsp-940-foreach-route-oracles/integer-compiled.tcl`
- Working directory: `/workspace/tcl-lsp`
- Exit status: 0
- Standard output:

```text-visible
patchlevel 0.84-10-g82fb4d0 have_rep 0
visited 1
```

- Standard error:

```text-visible
⟦EOF⟧
```
