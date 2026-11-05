# BIG-IP Tcl log: nh20261012pat3, tmm-traffic-event-3

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **list-prefix:** Does a list command prefix preserve a command name containing spaces and its fixed multiword argument when eval invokes it with an additional argument?

- **namespace-captured-prefix:** Does a namespace code prefix retain the defining namespace when evaluated elsewhere, including after a same-named global command is defined, and preserve its argument elements?

- **oo-prefix-deleted-near-miss:** What happens to a command prefix containing an object's self command after that object is destroyed: can it still invoke the method or does command lookup fail?

## Assumptions

- **list-prefix:** Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.
- **namespace-captured-prefix:** Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.
- **oo-prefix-deleted-near-miss:** Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-traffic-event-3](fixtures/bigip-tmm-traffic-event-3.md)

- run: `nh20261012pat3`
- case: `oo-prefix-deleted-near-miss`
- case.2: `namespace-captured-prefix`
- case.3: `list-prefix`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012pat37_list_prefix {}
namespace eval ::__tcl_lsp_probe_nh20261012pat37_list_prefix {}
proc {::__tcl_lsp_probe_nh20261012pat37_list_prefix::two words} {fixed value} {list $fixed $value}
set target {::__tcl_lsp_probe_nh20261012pat37_list_prefix::two words}
set prefix [list $target {a b}]
```

### nh_operation

```tcl-visible
eval [linsert $prefix end {c d}]
```

### nh_cleanup

```tcl-visible
if {[namespace exists ::__tcl_lsp_probe_nh20261012pat37_list_prefix]} {namespace delete ::__tcl_lsp_probe_nh20261012pat37_list_prefix}
```

### nh_setup.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012pat37_namespace_captured_prefix {}
namespace eval ::__tcl_lsp_probe_nh20261012pat37_namespace_captured_prefix {
    proc __tcl_lsp_probe_nh20261012pat37_namespace_captured_prefix_emit {value args} {list local $value $args}
    set prefix [namespace code [list __tcl_lsp_probe_nh20261012pat37_namespace_captured_prefix_emit {a b}]]
}
proc ::__tcl_lsp_probe_nh20261012pat37_namespace_captured_prefix_emit {args} {return global}
```

### nh_operation.2

```tcl-visible
eval [linsert $::__tcl_lsp_probe_nh20261012pat37_namespace_captured_prefix::prefix end {c d}]
```

### nh_cleanup.2

```tcl-visible
if {[llength [info commands ::__tcl_lsp_probe_nh20261012pat37_namespace_captured_prefix_emit]]} {rename ::__tcl_lsp_probe_nh20261012pat37_namespace_captured_prefix_emit {}}
if {[namespace exists ::__tcl_lsp_probe_nh20261012pat37_namespace_captured_prefix]} {namespace delete ::__tcl_lsp_probe_nh20261012pat37_namespace_captured_prefix}
```

### nh_setup.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012pat37_oo_prefix_deleted_near_miss {}
unset -nocomplain object
oo::class create ::__tcl_lsp_probe_nh20261012pat37_oo_prefix_deleted_near_miss::Class {
    method emit {value} {list ok $value}
    method prefix {} {list [self] emit {a b}}
}
set object [::__tcl_lsp_probe_nh20261012pat37_oo_prefix_deleted_near_miss::Class new]
set prefix [$object prefix]
$object destroy
catch {eval $prefix} result
```

### nh_operation.3

```tcl-visible
string match {invalid command name *} $result
```

### nh_cleanup.3

```tcl-visible
if {[info exists object]} {catch {$object destroy}}
if {[namespace exists ::__tcl_lsp_probe_nh20261012pat37_oo_prefix_deleted_near_miss]} {namespace delete ::__tcl_lsp_probe_nh20261012pat37_oo_prefix_deleted_near_miss}
```

## Results

### tmm-traffic-event, unit 0:0, invocation 0; tmm-traffic-event, unit 0:0, invocation 1; tmm-traffic-event, unit 0:0, invocation 10; tmm-traffic-event, unit 0:0, invocation 11; tmm-traffic-event, unit 0:0, invocation 12; tmm-traffic-event, unit 0:0, invocation 13; tmm-traffic-event, unit 0:0, invocation 14; tmm-traffic-event, unit 0:0, invocation 15; tmm-traffic-event, unit 0:0, invocation 2; tmm-traffic-event, unit 0:0, invocation 3; tmm-traffic-event, unit 0:0, invocation 4; tmm-traffic-event, unit 0:0, invocation 5; tmm-traffic-event, unit 0:0, invocation 6; tmm-traffic-event, unit 0:0, invocation 7; tmm-traffic-event, unit 0:0, invocation 8

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- list-prefix, setup, completion 0, result:

```text-visible
{::__tcl_lsp_probe_nh20261012pat37_list_prefix::two words} {a b}⟦EOF⟧
```

- list-prefix, lookup, completion 0, result:

```text-visible
{a b} {c d}⟦EOF⟧
```

- list-prefix, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- namespace-captured-prefix, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, result:

```text-visible
can't read "::__tcl_lsp_probe_nh20261012pat37_namespace_captured_prefix::prefix": no such variable⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, errorinfo:

```text-visible
can't read "::__tcl_lsp_probe_nh20261012pat37_namespace_captured_prefix::prefix": no such variable
    while executing
"linsert $::__tcl_lsp_probe_nh20261012pat37_namespace_captured_prefix::prefix end {c d}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace-captured-prefix, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, result:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, errorinfo:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__tcl_lsp_probe_nh20261012pat37_oo_prefix_deleted_near_miss::Class {
    method emit {value} {list ok $value}
    method prefix {} ..."
    ("eval" body line 3)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- oo-prefix-deleted-near-miss, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix: intended operation reached.

- namespace-captured-prefix: intended operation reached.

- oo-prefix-deleted-near-miss: intended operation not reached.

### tmm-traffic-event, unit 0:0, invocation 9

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- list-prefix, setup, completion 0, result:

```text-visible
{::__tcl_lsp_probe_nh20261012pat37_list_prefix::two words} {a b}⟦EOF⟧
```

- list-prefix, lookup, completion 0, result:

```text-visible
{a b} {c d}⟦EOF⟧
```

- list-prefix, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- namespace-captured-prefix, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, result:

```text-visible
can't read "::__tcl_lsp_probe_nh20261012pat37_namespace_captured_prefix::prefix": no such variable⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, errorinfo:

```text-visible
can't read "::__tcl_lsp_probe_nh20261012pat37_namespace_captured_prefix::prefix": no such variable
    while executing
"linsert $::__tcl_lsp_probe_nh20261012pat37_namespace_captured_prefix::prefix end {c d}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace-captured-prefix, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, result:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, errorinfo:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__tcl_lsp_probe_nh20261012pat37_oo_prefix_deleted_near_miss::Class {
    method emit {value} {list ok $value}
    method prefix {} ..."
    ("eval" body line 3)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- oo-prefix-deleted-near-miss, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix: intended operation reached.

- namespace-captured-prefix: intended operation reached.

- oo-prefix-deleted-near-miss: intended operation not reached.
