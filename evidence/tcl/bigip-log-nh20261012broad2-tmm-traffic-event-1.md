# BIG-IP Tcl log: nh20261012broad2, tmm-traffic-event-1

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **absolute-bypasses-local:** Bare and relative-qualified command lookup: current, path, global order; qualifier namespace exists without target; no ancestor walk; absolute bypass. Does path exist?

- **colon-run-and-empty-tail:** Colon runs, empty command/tail, lone colon names, namespace segment colons and duplicate display FQNs: which slots are actually reached?

- **no-ancestor-walk:** Bare and relative-qualified command lookup: current, path, global order; qualifier namespace exists without target; no ancestor walk; absolute bypass. Does path exist?

## Assumptions

- **absolute-bypasses-local:** Absolute command spelling; clean ::__tcl_lsp_probe_nh20261012broad27_absolute_bypasses_local_target too.
- **colon-run-and-empty-tail:** NH09 adapted names; preserve separate substitution reachability.
- **no-ancestor-walk:** NH04 adapted names; unique tail avoids accidental global fallback.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-traffic-event-3](fixtures/bigip-tmm-traffic-event-3.md)

- run: `nh20261012broad2`
- case: `colon-run-and-empty-tail`
- case.2: `absolute-bypasses-local`
- case.3: `no-ancestor-walk`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk {}
namespace eval ::__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk {proc __tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent {} {return ANCESTOR}; namespace eval child {}}
```

### nh_operation

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk::child {__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent}
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad27_no_ancestor_walk* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk*] [info commands ::__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk*] [info vars ::__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk*]⟦EOF⟧
```

### nh_setup.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_absolute_bypasses_local {}
namespace eval ::__tcl_lsp_probe_nh20261012broad27_absolute_bypasses_local_target {proc p {} {return ABSOLUTE}}
namespace eval ::__tcl_lsp_probe_nh20261012broad27_absolute_bypasses_local::__tcl_lsp_probe_nh20261012broad27_absolute_bypasses_local_target {proc p {} {return LOCAL}}
```

### nh_operation.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_absolute_bypasses_local {::__tcl_lsp_probe_nh20261012broad27_absolute_bypasses_local_target::p}
```

### nh_cleanup.2

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad27_absolute_bypasses_local* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_absolute_bypasses_local*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad27_absolute_bypasses_local*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad27_absolute_bypasses_local*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_absolute_bypasses_local*] [info commands ::__tcl_lsp_probe_nh20261012broad27_absolute_bypasses_local*] [info vars ::__tcl_lsp_probe_nh20261012broad27_absolute_bypasses_local*]⟦EOF⟧
```

### nh_setup.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_colon_run_and_empty_tail {}
namespace eval ::__tcl_lsp_probe_nh20261012broad27_colon_run_and_empty_tail {proc p {} {return NORMAL}; proc {} {} {return EMPTY_TAIL}}
```

### nh_operation.3

```tcl-visible
list [::__tcl_lsp_probe_nh20261012broad27_colon_run_and_empty_tail:::p] [::__tcl_lsp_probe_nh20261012broad27_colon_run_and_empty_tail:::]
```

### nh_cleanup.3

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad27_colon_run_and_empty_tail* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_colon_run_and_empty_tail*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad27_colon_run_and_empty_tail*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad27_colon_run_and_empty_tail*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_colon_run_and_empty_tail*] [info commands ::__tcl_lsp_probe_nh20261012broad27_colon_run_and_empty_tail*] [info vars ::__tcl_lsp_probe_nh20261012broad27_colon_run_and_empty_tail*]⟦EOF⟧
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

- no-ancestor-walk, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, result:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    while executing
"__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk::child" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk::child {__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- no-ancestor-walk, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- absolute-bypasses-local, lookup, completion 0, result:

```text-visible
ABSOLUTE⟦EOF⟧
```

- absolute-bypasses-local, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- colon-run-and-empty-tail, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- colon-run-and-empty-tail, lookup, completion 0, result:

```text-visible
NORMAL EMPTY_TAIL⟦EOF⟧
```

- colon-run-and-empty-tail, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local: intended operation reached.

- colon-run-and-empty-tail: intended operation reached.

- no-ancestor-walk: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 9

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- no-ancestor-walk, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, result:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    while executing
"__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk::child" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk::child {__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- no-ancestor-walk, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- absolute-bypasses-local, lookup, completion 0, result:

```text-visible
ABSOLUTE⟦EOF⟧
```

- absolute-bypasses-local, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- colon-run-and-empty-tail, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- colon-run-and-empty-tail, lookup, completion 0, result:

```text-visible
NORMAL EMPTY_TAIL⟦EOF⟧
```

- colon-run-and-empty-tail, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local: intended operation reached.

- colon-run-and-empty-tail: intended operation reached.

- no-ancestor-walk: intended operation reached.
