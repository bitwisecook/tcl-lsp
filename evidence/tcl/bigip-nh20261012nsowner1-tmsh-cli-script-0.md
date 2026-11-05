# BIG-IP tmsh Tcl: nh20261012nsowner1, tmsh-cli-script-0

## Question

This script contains the following cases, which investigate Tcl name handling relevant to the LSP, compiler and VM. Each observation is limited to its own case and this execution context.

- **namespace-definition-owner:** Bare and relative-qualified command lookup: current, path, global order; qualifier namespace exists without target; no ancestor walk; absolute bypass. Does path exist?

## Assumptions

- **namespace-definition-owner:** Unique namespace/command-tail control for namespace eval definition ownership in the actual wrapper frame; requested namespace, caller namespace and global candidates checked separately. It supplies no answer to an unchanged original corpus program. Intended operation: reached.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmsh-cli-script-1](fixtures/bigip-tmsh-cli-script-1.md)

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

### BIG-IP tmsh CLI script

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmsh run cli script.
- Exit status: 0
- Standard output: decoded below.

- Standard error: empty.

The exit status and streams are from the recorded tmsh invocation. The entries below decode complete NHO payloads from its standard output.

#### namespace-definition-owner

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
absent execution-namespace ::script candidates {::__tcl_lsp_probe_nh20261012nsowner1_namespace::__tcl_lsp_probe_nh20261012nsowner1_target ::script::__tcl_lsp_probe_nh20261012nsowner1_target ::__tcl_lsp_probe_nh20261012nsowner1_target}⟦EOF⟧
```

- lookup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
execution-namespace ::script inside-namespace ::__tcl_lsp_probe_nh20261012nsowner1_namespace definitions {{head ::__tcl_lsp_probe_nh20261012nsowner1_namespace::__tcl_lsp_probe_nh20261012nsowner1_target commands {} procs {} invocation 1 {invalid command name "::__tcl_lsp_probe_nh20261012nsowner1_namespace::__tcl_lsp_probe_nh20261012nsowner1_target"} body 1 {"::__tcl_lsp_probe_nh20261012nsowner1_namespace::__tcl_lsp_probe_nh20261012nsowner1_target" isn't a procedure}} {head ::script::__tcl_lsp_probe_nh20261012nsowner1_target commands {} procs {} invocation 1 {invalid command name "::script::__tcl_lsp_probe_nh20261012nsowner1_target"} body 1 {"::script::__tcl_lsp_probe_nh20261012nsowner1_target" isn't a procedure}} {head ::__tcl_lsp_probe_nh20261012nsowner1_target commands ::__tcl_lsp_probe_nh20261012nsowner1_target procs ::__tcl_lsp_probe_nh20261012nsowner1_target invocation 0 TARGET body 0 {return TARGET}}}⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
namespace 0 candidates {{head ::__tcl_lsp_probe_nh20261012nsowner1_namespace::__tcl_lsp_probe_nh20261012nsowner1_target commands {} procs {} invocation 1 {invalid command name "::__tcl_lsp_probe_nh20261012nsowner1_namespace::__tcl_lsp_probe_nh20261012nsowner1_target"}} {head ::script::__tcl_lsp_probe_nh20261012nsowner1_target commands {} procs {} invocation 1 {invalid command name "::script::__tcl_lsp_probe_nh20261012nsowner1_target"}} {head ::__tcl_lsp_probe_nh20261012nsowner1_target commands {} procs {} invocation 1 {invalid command name "::__tcl_lsp_probe_nh20261012nsowner1_target"}}}⟦EOF⟧
```
