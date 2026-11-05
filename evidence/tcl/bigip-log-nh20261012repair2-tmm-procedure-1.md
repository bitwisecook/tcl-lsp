# BIG-IP Tcl log: nh20261012repair2, tmm-procedure-1

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **child-provider-and-proc-owner:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **child-tcltest-real-package:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **parent-proc-rename-owner:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

## Assumptions

- **child-provider-and-proc-owner:** Independent capability/ownership diagnostic; no original lookup answer
- **child-tcltest-real-package:** Independent capability/ownership diagnostic; no original lookup answer
- **parent-proc-rename-owner:** Independent capability/ownership diagnostic; no original lookup answer

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-procedure-6](fixtures/bigip-tmm-procedure-6.md)

- run: `nh20261012repair2`
- case: `child-provider-and-proc-owner`
- case.2: `child-tcltest-real-package`
- case.3: `parent-proc-rename-owner`

### nh_setup

```tcl-visible
set nh_child [interp create]⟦EOF⟧
```

### nh_operation

```tcl-visible
interp eval $nh_child {list [info patchlevel] [info tclversion] [namespace current] [namespace eval ::__tcl_lsp_probe_nh20261012repair2 {proc tail {} {namespace current}; list [info commands ::__tcl_lsp_probe_nh20261012repair2::tail] [info commands ::tail] [catch {tail} nh_x] $nh_x}]}⟦EOF⟧
```

### nh_cleanup

```tcl-visible
interp delete $nh_child⟦EOF⟧
```

### nh_setup.2

```tcl-visible
set nh_child [interp create]⟦EOF⟧
```

### nh_operation.2

```tcl-visible
interp eval $nh_child {list [package require tcltest] [info commands ::tcltest::test] [info commands ::test]}⟦EOF⟧
```

### nh_cleanup.2

```tcl-visible
interp delete $nh_child⟦EOF⟧
```

### nh_setup.3

```tcl-visible
if {[namespace exists ::__tcl_lsp_probe_nh20261012repair2] || [llength [info commands ::__tcl_lsp_probe_nh20261012repair2_tail]]} {error ownership-collision}; namespace eval ::__tcl_lsp_probe_nh20261012repair2 {}⟦EOF⟧
```

### nh_operation.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012repair2 {proc ::__tcl_lsp_probe_nh20261012repair2_tail {} {variable debug; namespace current}}; set nh_before [list [info commands ::__tcl_lsp_probe_nh20261012repair2::__tcl_lsp_probe_nh20261012repair2_tail] [info commands ::__tcl_lsp_probe_nh20261012repair2_tail]]; rename ::__tcl_lsp_probe_nh20261012repair2_tail ::__tcl_lsp_probe_nh20261012repair2::tail; list $nh_before [info commands ::__tcl_lsp_probe_nh20261012repair2::tail] [catch {::__tcl_lsp_probe_nh20261012repair2::tail} nh_x] $nh_x⟦EOF⟧
```

### nh_cleanup.3

```tcl-visible
if {[llength [info commands ::__tcl_lsp_probe_nh20261012repair2_tail]]} {rename ::__tcl_lsp_probe_nh20261012repair2_tail {}}; namespace delete ::__tcl_lsp_probe_nh20261012repair2⟦EOF⟧
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

- child-provider-and-proc-owner, setup, completion 0, result:

```text-visible
interp0⟦EOF⟧
```

- child-provider-and-proc-owner, lookup, completion 0, result:

```text-visible
8.4.6 8.4 :: {::__tcl_lsp_probe_nh20261012repair2::tail {} 0 ::__tcl_lsp_probe_nh20261012repair2}⟦EOF⟧
```

- child-provider-and-proc-owner, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- child-tcltest-real-package, setup, completion 0, result:

```text-visible
interp0⟦EOF⟧
```

- child-tcltest-real-package, lookup, completion 1, result:

```text-visible
can't find package tcltest⟦EOF⟧
```

- child-tcltest-real-package, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- child-tcltest-real-package, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- child-tcltest-real-package, lookup, completion 1, errorinfo:

```text-visible
can't find package tcltest
    while executing
"package require tcltest"
    invoked from within
"interp eval $nh_child {list [package require tcltest] [info commands ::tcltest::test] [info commands ::test]}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- child-tcltest-real-package, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- parent-proc-rename-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- parent-proc-rename-owner, lookup, completion 0, result:

```text-visible
{{} ::__tcl_lsp_probe_nh20261012repair2_tail} ::__tcl_lsp_probe_nh20261012repair2::tail 0 ::__tcl_lsp_probe_nh20261012repair2⟦EOF⟧
```

- parent-proc-rename-owner, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- child-provider-and-proc-owner: intended operation reached.

- child-tcltest-real-package: intended operation reached.

- parent-proc-rename-owner: intended operation reached.

### tmm-procedure, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- child-provider-and-proc-owner, setup, completion 0, result:

```text-visible
interp0⟦EOF⟧
```

- child-provider-and-proc-owner, lookup, completion 0, result:

```text-visible
8.4.6 8.4 :: {::__tcl_lsp_probe_nh20261012repair2::tail {} 0 ::__tcl_lsp_probe_nh20261012repair2}⟦EOF⟧
```

- child-provider-and-proc-owner, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- child-tcltest-real-package, setup, completion 0, result:

```text-visible
interp0⟦EOF⟧
```

- child-tcltest-real-package, lookup, completion 1, result:

```text-visible
can't find package tcltest⟦EOF⟧
```

- child-tcltest-real-package, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- child-tcltest-real-package, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- child-tcltest-real-package, lookup, completion 1, errorinfo:

```text-visible
can't find package tcltest
    while executing
"package require tcltest"
    invoked from within
"interp eval $nh_child {list [package require tcltest] [info commands ::tcltest::test] [info commands ::test]}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- child-tcltest-real-package, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- parent-proc-rename-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- parent-proc-rename-owner, lookup, completion 0, result:

```text-visible
{{} ::__tcl_lsp_probe_nh20261012repair2_tail} ::__tcl_lsp_probe_nh20261012repair2::tail 0 ::__tcl_lsp_probe_nh20261012repair2⟦EOF⟧
```

- parent-proc-rename-owner, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- child-provider-and-proc-owner: intended operation reached.

- child-tcltest-real-package: intended operation reached.

- parent-proc-rename-owner: intended operation reached.
