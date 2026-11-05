# BIG-IP Tcl log: nh20261012nsowner1, tmm-procedure-0

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **namespace-definition-owner:** Bare and relative-qualified command lookup: current, path, global order; qualifier namespace exists without target; no ancestor walk; absolute bypass. Does path exist?

## Assumptions

- **namespace-definition-owner:** Unique namespace/command-tail control for namespace eval definition ownership in the actual wrapper frame; requested namespace, caller namespace and global candidates checked separately. It supplies no answer to an unchanged original corpus program.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-procedure-1](fixtures/bigip-tmm-procedure-1.md)

- run: `nh20261012nsowner1`
- case: `namespace-definition-owner`

### nh_setup

```tcl-visible
set nh_owner_n namespace
set nh_owner_scope [$nh_owner_n current]
set nh_owner_candidates [list ::__tcl_lsp_probe_nh20261012nsowner1_namespace::__tcl_lsp_probe_nh20261012nsowner1_target [string trimright $nh_owner_scope :]::__tcl_lsp_probe_nh20261012nsowner1_target ::__tcl_lsp_probe_nh20261012nsowner1_target]
if {[$nh_owner_n exists ::__tcl_lsp_probe_nh20261012nsowner1_namespace] || [llength [info commands ::__tcl_lsp_probe_nh20261012nsowner1_namespace]]} {error {namespace ownership collision}}
foreach nh_owner_head $nh_owner_candidates {if {[llength [info commands $nh_owner_head]] || [llength [info procs $nh_owner_head]]} {error [list command ownership collision $nh_owner_head]}}
list absent execution-namespace $nh_owner_scope candidates $nh_owner_candidates
```

### nh_operation

```tcl-visible
set nh_owner_inside [namespace eval ::__tcl_lsp_probe_nh20261012nsowner1_namespace {
proc __tcl_lsp_probe_nh20261012nsowner1_target {} {return TARGET}
namespace current
}]
set nh_owner_rows {}
foreach nh_owner_head $nh_owner_candidates {
set nh_owner_code [catch {eval [list $nh_owner_head]} nh_owner_value]
set nh_owner_body_code [catch {info body $nh_owner_head} nh_owner_body]
lappend nh_owner_rows [list head $nh_owner_head commands [info commands $nh_owner_head] procs [info procs $nh_owner_head] invocation $nh_owner_code $nh_owner_value body $nh_owner_body_code $nh_owner_body]
}
list execution-namespace $nh_owner_scope inside-namespace $nh_owner_inside definitions $nh_owner_rows
```

### nh_cleanup

```tcl-visible
if {$nh_setup_code != 0} {return {ownership-not-acquired}}
foreach nh_owner_head $nh_owner_candidates {if {[llength [info commands $nh_owner_head]]} {rename $nh_owner_head {}}}
if {[namespace exists ::__tcl_lsp_probe_nh20261012nsowner1_namespace]} {namespace delete ::__tcl_lsp_probe_nh20261012nsowner1_namespace}
set nh_owner_remaining {}
foreach nh_owner_head $nh_owner_candidates {
set nh_owner_after_code [catch {eval [list $nh_owner_head]} nh_owner_after_value]
lappend nh_owner_remaining [list head $nh_owner_head commands [info commands $nh_owner_head] procs [info procs $nh_owner_head] invocation $nh_owner_after_code $nh_owner_after_value]
}
list namespace [namespace exists ::__tcl_lsp_probe_nh20261012nsowner1_namespace] candidates $nh_owner_remaining
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

- namespace-definition-owner, setup, completion 0, result:

```text-visible
absent execution-namespace :: candidates {::__tcl_lsp_probe_nh20261012nsowner1_namespace::__tcl_lsp_probe_nh20261012nsowner1_target ::__tcl_lsp_probe_nh20261012nsowner1_target ::__tcl_lsp_probe_nh20261012nsowner1_target}⟦EOF⟧
```

- namespace-definition-owner, lookup, completion 0, result:

```text-visible
execution-namespace :: inside-namespace ::__tcl_lsp_probe_nh20261012nsowner1_namespace definitions {{head ::__tcl_lsp_probe_nh20261012nsowner1_namespace::__tcl_lsp_probe_nh20261012nsowner1_target commands ::__tcl_lsp_probe_nh20261012nsowner1_namespace::__tcl_lsp_probe_nh20261012nsowner1_target procs ::__tcl_lsp_probe_nh20261012nsowner1_namespace::__tcl_lsp_probe_nh20261012nsowner1_target invocation 0 TARGET body 0 {return TARGET}} {head ::__tcl_lsp_probe_nh20261012nsowner1_target commands {} procs {} invocation 1 {invalid command name "::__tcl_lsp_probe_nh20261012nsowner1_target"} body 1 {"::__tcl_lsp_probe_nh20261012nsowner1_target" isn't a procedure}} {head ::__tcl_lsp_probe_nh20261012nsowner1_target commands {} procs {} invocation 1 {invalid command name "::__tcl_lsp_probe_nh20261012nsowner1_target"} body 1 {"::__tcl_lsp_probe_nh20261012nsowner1_target" isn't a procedure}}}⟦EOF⟧
```

- namespace-definition-owner, runtime-cleanup, completion 0, result:

```text-visible
namespace 0 candidates {{head ::__tcl_lsp_probe_nh20261012nsowner1_namespace::__tcl_lsp_probe_nh20261012nsowner1_target commands {} procs {} invocation 1 {invalid command name "::__tcl_lsp_probe_nh20261012nsowner1_namespace::__tcl_lsp_probe_nh20261012nsowner1_target"}} {head ::__tcl_lsp_probe_nh20261012nsowner1_target commands {} procs {} invocation 1 {invalid command name "::__tcl_lsp_probe_nh20261012nsowner1_target"}} {head ::__tcl_lsp_probe_nh20261012nsowner1_target commands {} procs {} invocation 1 {invalid command name "::__tcl_lsp_probe_nh20261012nsowner1_target"}}}⟦EOF⟧
```

- namespace-definition-owner: intended operation reached.

### tmm-procedure, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- namespace-definition-owner, setup, completion 0, result:

```text-visible
absent execution-namespace :: candidates {::__tcl_lsp_probe_nh20261012nsowner1_namespace::__tcl_lsp_probe_nh20261012nsowner1_target ::__tcl_lsp_probe_nh20261012nsowner1_target ::__tcl_lsp_probe_nh20261012nsowner1_target}⟦EOF⟧
```

- namespace-definition-owner, lookup, completion 0, result:

```text-visible
execution-namespace :: inside-namespace ::__tcl_lsp_probe_nh20261012nsowner1_namespace definitions {{head ::__tcl_lsp_probe_nh20261012nsowner1_namespace::__tcl_lsp_probe_nh20261012nsowner1_target commands ::__tcl_lsp_probe_nh20261012nsowner1_namespace::__tcl_lsp_probe_nh20261012nsowner1_target procs ::__tcl_lsp_probe_nh20261012nsowner1_namespace::__tcl_lsp_probe_nh20261012nsowner1_target invocation 0 TARGET body 0 {return TARGET}} {head ::__tcl_lsp_probe_nh20261012nsowner1_target commands {} procs {} invocation 1 {invalid command name "::__tcl_lsp_probe_nh20261012nsowner1_target"} body 1 {"::__tcl_lsp_probe_nh20261012nsowner1_target" isn't a procedure}} {head ::__tcl_lsp_probe_nh20261012nsowner1_target commands {} procs {} invocation 1 {invalid command name "::__tcl_lsp_probe_nh20261012nsowner1_target"} body 1 {"::__tcl_lsp_probe_nh20261012nsowner1_target" isn't a procedure}}}⟦EOF⟧
```

- namespace-definition-owner, runtime-cleanup, completion 0, result:

```text-visible
namespace 0 candidates {{head ::__tcl_lsp_probe_nh20261012nsowner1_namespace::__tcl_lsp_probe_nh20261012nsowner1_target commands {} procs {} invocation 1 {invalid command name "::__tcl_lsp_probe_nh20261012nsowner1_namespace::__tcl_lsp_probe_nh20261012nsowner1_target"}} {head ::__tcl_lsp_probe_nh20261012nsowner1_target commands {} procs {} invocation 1 {invalid command name "::__tcl_lsp_probe_nh20261012nsowner1_target"}} {head ::__tcl_lsp_probe_nh20261012nsowner1_target commands {} procs {} invocation 1 {invalid command name "::__tcl_lsp_probe_nh20261012nsowner1_target"}}}⟦EOF⟧
```

- namespace-definition-owner: intended operation reached.
