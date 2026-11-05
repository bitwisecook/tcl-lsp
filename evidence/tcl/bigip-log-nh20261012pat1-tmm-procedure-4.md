# BIG-IP Tcl log: nh20261012pat1, tmm-procedure-4

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **oo-self-prefix:** Can a TclOO method return a list prefix containing its own object command and a multiword argument for later method invocation?

- **string-prefix-near-miss:** What happens when a command name containing spaces is interpolated into an eval script string rather than preserved as a list command-prefix element?

- **tcltest-shared-lifecycle:** Do tcltest setup, body and cleanup share the expected namespace variable state, and is the setup variable absent after cleanup?

## Assumptions

- **oo-self-prefix:** Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.
- **string-prefix-near-miss:** Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.
- **tcltest-shared-lifecycle:** Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-procedure-3](fixtures/bigip-tmm-procedure-3.md)

- run: `nh20261012pat1`
- case: `tcltest-shared-lifecycle`
- case.2: `string-prefix-near-miss`
- case.3: `oo-self-prefix`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012pat16_oo_self_prefix {}
unset -nocomplain object
oo::class create ::__tcl_lsp_probe_nh20261012pat16_oo_self_prefix::Class {
    method emit {value} {list ok $value}
    method prefix {} {list [self] emit {a b}}
}
set object [::__tcl_lsp_probe_nh20261012pat16_oo_self_prefix::Class new]
```

### nh_operation

```tcl-visible
eval [$object prefix]
```

### nh_cleanup

```tcl-visible
if {[info exists object]} {catch {$object destroy}}
if {[namespace exists ::__tcl_lsp_probe_nh20261012pat16_oo_self_prefix]} {namespace delete ::__tcl_lsp_probe_nh20261012pat16_oo_self_prefix}
```

### nh_setup.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012pat16_string_prefix_near_miss {}
namespace eval ::__tcl_lsp_probe_nh20261012pat16_string_prefix_near_miss {}
proc {::__tcl_lsp_probe_nh20261012pat16_string_prefix_near_miss::two words} {fixed value} {list $fixed $value}
set target {::__tcl_lsp_probe_nh20261012pat16_string_prefix_near_miss::two words}
```

### nh_operation.2

```tcl-visible
eval "$target {a b} {c d}"
```

### nh_cleanup.2

```tcl-visible
if {[namespace exists ::__tcl_lsp_probe_nh20261012pat16_string_prefix_near_miss]} {namespace delete ::__tcl_lsp_probe_nh20261012pat16_string_prefix_near_miss}
```

### nh_setup.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012pat16_tcltest_shared_lifecycle {}
package require tcltest
namespace eval ::__tcl_lsp_probe_nh20261012pat16_tcltest_shared_lifecycle {
    namespace import ::tcltest::*
    set observed {}
    test name-corpus-1 {} -setup {
        set payload {a b}
    } -body {
        lappend observed [set payload]
    } -cleanup {
        lappend observed [set payload]
        unset payload
    } -result {{a b}}
}
```

### nh_operation.3

```tcl-visible
list $::__tcl_lsp_probe_nh20261012pat16_tcltest_shared_lifecycle::observed [info exists ::__tcl_lsp_probe_nh20261012pat16_tcltest_shared_lifecycle::payload]
```

### nh_cleanup.3

```tcl-visible
if {[namespace exists ::__tcl_lsp_probe_nh20261012pat16_tcltest_shared_lifecycle]} {namespace delete ::__tcl_lsp_probe_nh20261012pat16_tcltest_shared_lifecycle}
```

## Results

### tmm-procedure, unit 0:0, invocation once; tmm-procedure, unit 0:1, invocation once; tmm-procedure, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- oo-self-prefix, setup, completion 1, result:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- oo-self-prefix, setup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- oo-self-prefix, setup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- oo-self-prefix, setup, completion 1, errorinfo:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__tcl_lsp_probe_nh20261012pat16_oo_self_prefix::Class {
    method emit {value} {list ok $value}
    method prefix {} {list [self] ..."
    ("eval" body line 3)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- oo-self-prefix, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- string-prefix-near-miss, setup, completion 0, result:

```text-visible
::__tcl_lsp_probe_nh20261012pat16_string_prefix_near_miss::two words⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, result:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012pat16_string_prefix_near_miss::two"⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorinfo:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012pat16_string_prefix_near_miss::two"
    while executing
"::__tcl_lsp_probe_nh20261012pat16_string_prefix_near_miss::two words {a b} {c d}"
    ("eval" body line 1)
    invoked from within
"eval "$target {a b} {c d}""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- string-prefix-near-miss, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- tcltest-shared-lifecycle, setup, completion 1, result:

```text-visible
can't find package tcltest⟦EOF⟧
```

- tcltest-shared-lifecycle, setup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- tcltest-shared-lifecycle, setup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- tcltest-shared-lifecycle, setup, completion 1, errorinfo:

```text-visible
can't find package tcltest
    while executing
"package require tcltest"
    ("eval" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- tcltest-shared-lifecycle, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- oo-self-prefix: intended operation not reached.

- string-prefix-near-miss: intended operation reached.

- tcltest-shared-lifecycle: intended operation not reached.

### tmm-procedure, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- oo-self-prefix, setup, completion 1, result:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- oo-self-prefix, setup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- oo-self-prefix, setup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- oo-self-prefix, setup, completion 1, errorinfo:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__tcl_lsp_probe_nh20261012pat16_oo_self_prefix::Class {
    method emit {value} {list ok $value}
    method prefix {} {list [self] ..."
    ("eval" body line 3)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- oo-self-prefix, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- string-prefix-near-miss, setup, completion 0, result:

```text-visible
::__tcl_lsp_probe_nh20261012pat16_string_prefix_near_miss::two words⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, result:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012pat16_string_prefix_near_miss::two"⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorinfo:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012pat16_string_prefix_near_miss::two"
    while executing
"::__tcl_lsp_probe_nh20261012pat16_string_prefix_near_miss::two words {a b} {c d}"
    ("eval" body line 1)
    invoked from within
"eval "$target {a b} {c d}""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- string-prefix-near-miss, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- tcltest-shared-lifecycle, setup, completion 1, result:

```text-visible
can't find package tcltest⟦EOF⟧
```

- tcltest-shared-lifecycle, setup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- tcltest-shared-lifecycle, setup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- tcltest-shared-lifecycle, setup, completion 1, errorinfo:

```text-visible
can't find package tcltest
    while executing
"package require tcltest"
    ("eval" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- tcltest-shared-lifecycle, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- oo-self-prefix: intended operation not reached.

- string-prefix-near-miss: intended operation reached.

- tcltest-shared-lifecycle: intended operation not reached.
