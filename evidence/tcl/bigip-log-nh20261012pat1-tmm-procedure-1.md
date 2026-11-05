# BIG-IP Tcl log: nh20261012pat1, tmm-procedure-1

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **ensemble-prefix-map:** Does an ensemble subcommand mapping preserve the elements of a list command prefix, including a fixed argument containing a space, when passing a new argument?

- **factory-body-near-miss:** When a loop defines procedures using a braced body containing the loop variable, is that variable substituted during definition or looked up when the generated procedure executes?

- **finite-factory:** When a loop constructs procedure bodies as lists containing the current loop value, do the resulting commands retain their respective values?

## Assumptions

- **ensemble-prefix-map:** Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.
- **factory-body-near-miss:** Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.
- **finite-factory:** Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-procedure-2](fixtures/bigip-tmm-procedure-2.md)

- run: `nh20261012pat1`
- case: `factory-body-near-miss`
- case.2: `ensemble-prefix-map`
- case.3: `finite-factory`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012pat16_ensemble_prefix_map {}
namespace eval ::__tcl_lsp_probe_nh20261012pat16_ensemble_prefix_map {
    proc emit {fixed value} {list $fixed $value}
    namespace ensemble create -command facade -map [list send [list ::__tcl_lsp_probe_nh20261012pat16_ensemble_prefix_map::emit {a b}]]
}
```

### nh_operation

```tcl-visible
::__tcl_lsp_probe_nh20261012pat16_ensemble_prefix_map::facade send {c d}
```

### nh_cleanup

```tcl-visible
if {[namespace exists ::__tcl_lsp_probe_nh20261012pat16_ensemble_prefix_map]} {namespace delete ::__tcl_lsp_probe_nh20261012pat16_ensemble_prefix_map}
```

### nh_setup.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012pat16_factory_body_near_miss {}
namespace eval ::__tcl_lsp_probe_nh20261012pat16_factory_body_near_miss {
    foreach name {red blue} {proc $name {} {return $name}}
}
```

### nh_operation.2

```tcl-visible
::__tcl_lsp_probe_nh20261012pat16_factory_body_near_miss::red
```

### nh_cleanup.2

```tcl-visible
if {[namespace exists ::__tcl_lsp_probe_nh20261012pat16_factory_body_near_miss]} {namespace delete ::__tcl_lsp_probe_nh20261012pat16_factory_body_near_miss}
```

### nh_setup.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012pat16_finite_factory {}
namespace eval ::__tcl_lsp_probe_nh20261012pat16_finite_factory {
    foreach name {red blue} {proc $name {} [list return $name]}
}
```

### nh_operation.3

```tcl-visible
list [::__tcl_lsp_probe_nh20261012pat16_finite_factory::red] [::__tcl_lsp_probe_nh20261012pat16_finite_factory::blue]
```

### nh_cleanup.3

```tcl-visible
if {[namespace exists ::__tcl_lsp_probe_nh20261012pat16_finite_factory]} {namespace delete ::__tcl_lsp_probe_nh20261012pat16_finite_factory}
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

- ensemble-prefix-map, setup, completion 1, result:

```text-visible
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, errorinfo:

```text-visible
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace ensemble create -command facade -map [list send [list ::__tcl_lsp_probe_nh20261012pat16_ensemble_prefix_map::emit {a b}]]"
    (in namespace eval "::__tcl_lsp_probe_nh20261012pat16_ensemble_prefix_map" script line 3)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012pat16_ensemble_prefix_map {
    proc emit {fixed value} {list $fixed $value}
    namespace ensemble create ..."
    ("eval" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- ensemble-prefix-map, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- factory-body-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- factory-body-near-miss, lookup, completion 1, result:

```text-visible
can't read "name": no such variable⟦EOF⟧
```

- factory-body-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- factory-body-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- factory-body-near-miss, lookup, completion 1, errorinfo:

```text-visible
can't read "name": no such variable
    while executing
"return $name"
    (procedure "::__tcl_lsp_probe_nh20261012pat16_factory_body_near_miss::re..." line 1)
    invoked from within
"::__tcl_lsp_probe_nh20261012pat16_factory_body_near_miss::red"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- factory-body-near-miss, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- finite-factory, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- finite-factory, lookup, completion 0, result:

```text-visible
red blue⟦EOF⟧
```

- finite-factory, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- ensemble-prefix-map: intended operation not reached.

- factory-body-near-miss: intended operation reached.

- finite-factory: intended operation reached.

### tmm-procedure, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- ensemble-prefix-map, setup, completion 1, result:

```text-visible
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, errorinfo:

```text-visible
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace ensemble create -command facade -map [list send [list ::__tcl_lsp_probe_nh20261012pat16_ensemble_prefix_map::emit {a b}]]"
    (in namespace eval "::__tcl_lsp_probe_nh20261012pat16_ensemble_prefix_map" script line 3)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012pat16_ensemble_prefix_map {
    proc emit {fixed value} {list $fixed $value}
    namespace ensemble create ..."
    ("eval" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- ensemble-prefix-map, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- factory-body-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- factory-body-near-miss, lookup, completion 1, result:

```text-visible
can't read "name": no such variable⟦EOF⟧
```

- factory-body-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- factory-body-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- factory-body-near-miss, lookup, completion 1, errorinfo:

```text-visible
can't read "name": no such variable
    while executing
"return $name"
    (procedure "::__tcl_lsp_probe_nh20261012pat16_factory_body_near_miss::re..." line 1)
    invoked from within
"::__tcl_lsp_probe_nh20261012pat16_factory_body_near_miss::red"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- factory-body-near-miss, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- finite-factory, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- finite-factory, lookup, completion 0, result:

```text-visible
red blue⟦EOF⟧
```

- finite-factory, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- ensemble-prefix-map: intended operation not reached.

- factory-body-near-miss: intended operation reached.

- finite-factory: intended operation reached.
