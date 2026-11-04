# Tcl execution evidence

## Question

Does this three-iteration `foreach` script repeatedly change the same object between integer and list representations when `incr x` is followed by `lindex $x 0`? This supplies a genuine loop-conversion control for S102. It does not establish LSP or compiler diagnostic behaviour.

## Assumptions

The retained capture records the exact script, CLI argument vectors, process exit statuses and both process streams. Host platform: Not recorded. Compiler flags: Not recorded. Working directory: Not recorded. Ambient startup, trace and alias configuration: Not recorded.

The representation observer is available only in the captured Tcl 8.6, 9.0 and 9.1 runs. The Tcl 8.4, 8.5 and Jim runs establish the final result only; their lack of representation observations is not evidence of an internal type or object identity. Pointer equality is interpreted only within one recorded process.

The [raw capture](../../docs/LSP_RELEASE_SHIMMER_CYCLE_RECEIPT_2026-11-04.json) is retained. This document converts that capture; it does not claim a new execution.

## Script

```tcl-visible
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
set have_rep [expr {[llength [info commands ::tcl::unsupported::representation]] != 0}]
set records {}
set x 0
foreach i {1 2 3} {
    incr x
    if {$have_rep} {
        lappend records [list before [::tcl::unsupported::representation $x]]
    }
    lindex $x 0
    if {$have_rep} {
        lappend records [list after [::tcl::unsupported::representation $x]]
    }
}
puts [list patchlevel [info patchlevel] final $x observations [llength $records]]
foreach record $records {puts $record}
```

## Results

### Tcl 8.4.20

- Recorded version: 8.4.20
- Recorded input channel: command-line script file.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.4 /tmp/lsp-940-foreach-cycle-oracle.tcl`
- Exit status: 0
- Standard output:

```text-visible
patchlevel 8.4.20 final 3 observations 0
```

- Standard error:

```text-visible
⟦EOF⟧
```

### Tcl 8.5.19

- Recorded version: 8.5.19
- Recorded input channel: command-line script file.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.5 /tmp/lsp-940-foreach-cycle-oracle.tcl`
- Exit status: 0
- Standard output:

```text-visible
patchlevel 8.5.19 final 3 observations 0
```

- Standard error:

```text-visible
⟦EOF⟧
```

### Tcl 8.6.18

- Recorded version: 8.6.18
- Recorded input channel: command-line script file.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.6 /tmp/lsp-940-foreach-cycle-oracle.tcl`
- Exit status: 0
- Standard output:

```text-visible
patchlevel 8.6.18 final 3 observations 6
before {value is a int with a refcount of 2, object pointer at 0x557b0b760800, internal representation 0x1:0x557b0b75f8d0, no string representation}
after {value is a list with a refcount of 2, object pointer at 0x557b0b760800, internal representation 0x557b0b7725d0:(nil), string representation "1"}
before {value is a int with a refcount of 2, object pointer at 0x557b0b760800, internal representation 0x2:(nil), no string representation}
after {value is a list with a refcount of 2, object pointer at 0x557b0b760800, internal representation 0x557b0b774e20:(nil), string representation "2"}
before {value is a int with a refcount of 2, object pointer at 0x557b0b760800, internal representation 0x3:(nil), no string representation}
after {value is a list with a refcount of 2, object pointer at 0x557b0b760800, internal representation 0x557b0b7723d0:(nil), string representation "3"}
```

- Standard error:

```text-visible
⟦EOF⟧
```

### Tcl 9.0.4

- Recorded version: 9.0.4
- Recorded input channel: command-line script file.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh9.0 /tmp/lsp-940-foreach-cycle-oracle.tcl`
- Exit status: 0
- Standard output:

```text-visible
patchlevel 9.0.4 final 3 observations 6
before {value is a int with a refcount of 2, object pointer at 0x5584cf2c7ee0, internal representation 0x1:0x5584cf2ca4f0, no string representation}
after {value is a list with a refcount of 2, object pointer at 0x5584cf2c7ee0, internal representation 0x5584cf2f8660:0x0, string representation "1"}
before {value is a int with a refcount of 2, object pointer at 0x5584cf2c7ee0, internal representation 0x2:0x5584cf2c7eb0, no string representation}
after {value is a list with a refcount of 2, object pointer at 0x5584cf2c7ee0, internal representation 0x5584cf2fb6f0:0x0, string representation "2"}
before {value is a int with a refcount of 2, object pointer at 0x5584cf2c7ee0, internal representation 0x3:0x5584cf2c83f0, no string representation}
after {value is a list with a refcount of 2, object pointer at 0x5584cf2c7ee0, internal representation 0x5584cf2f8720:0x0, string representation "3"}
```

- Standard error:

```text-visible
⟦EOF⟧
```

### Tcl 9.1.0

- Recorded version: 9.1.0
- Recorded input channel: command-line script file.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh9.1 /tmp/lsp-940-foreach-cycle-oracle.tcl`
- Exit status: 0
- Standard output:

```text-visible
patchlevel 9.1.0 final 3 observations 6
before {value is a int with a refcount of 2, object pointer at 0x565364380060, internal representation 0x1:0x565364337fa0, no string representation}
after {value is a list with a refcount of 2, object pointer at 0x565364380060, internal representation 0x5653643b6c20:0x0, string representation "1"}
before {value is a int with a refcount of 2, object pointer at 0x565364380060, internal representation 0x2:0x565364337fa0, no string representation}
after {value is a list with a refcount of 2, object pointer at 0x565364380060, internal representation 0x5653643b69a0:0x0, string representation "2"}
before {value is a int with a refcount of 2, object pointer at 0x565364380060, internal representation 0x3:0x565364337fa0, no string representation}
after {value is a list with a refcount of 2, object pointer at 0x565364380060, internal representation 0x5653643b6e20:0x0, string representation "3"}
```

- Standard error:

```text-visible
⟦EOF⟧
```

### JimTcl 0.84-10-g82fb4d0

- Recorded version: 0.84-10-g82fb4d0
- Recorded input channel: command-line script file.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/jimtcl/jimsh /tmp/lsp-940-foreach-cycle-oracle.tcl`
- Exit status: 0
- Standard output:

```text-visible
patchlevel 0.84-10-g82fb4d0 final 3 observations 0
```

- Standard error:

```text-visible
⟦EOF⟧
```

