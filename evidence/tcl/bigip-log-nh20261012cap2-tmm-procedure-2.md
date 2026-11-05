# BIG-IP Tcl log: nh20261012cap2, tmm-procedure-2

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **namespace.ensemble:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **namespace.import:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **namespace.upvar:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

## Assumptions

- **namespace.ensemble:** Not recorded
- **namespace.import:** Not recorded
- **namespace.upvar:** Not recorded

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-procedure-2](fixtures/bigip-tmm-procedure-2.md)

- run: `nh20261012cap2`
- case: `namespace.ensemble`
- case.2: `namespace.import`
- case.3: `namespace.upvar`

### nh_setup

```tcl-visible
namespace eval ::__nh_nh20261012cap2_namespace_import {}⟦EOF⟧
```

### nh_operation

```tcl-visible
namespace eval ::__nh_nh20261012cap2_namespace_import {namespace import ::__absent_nh_*}⟦EOF⟧
```

### nh_cleanup

```tcl-visible
if {[namespace exists ::__nh_nh20261012cap2_namespace_import]} {namespace delete ::__nh_nh20261012cap2_namespace_import}; foreach nh_cmd [info commands ::__nh_nh20261012cap2_namespace_import_*] {rename $nh_cmd {}}; list [namespace exists ::__nh_nh20261012cap2_namespace_import] [info commands ::__nh_nh20261012cap2_namespace_import_*]⟦EOF⟧
```

### nh_setup.2

```tcl-visible
namespace eval ::__nh_nh20261012cap2_namespace_ensemble {}⟦EOF⟧
```

### nh_operation.2

```tcl-visible
namespace eval ::__nh_nh20261012cap2_namespace_ensemble {namespace ensemble create -command facade -map {}}⟦EOF⟧
```

### nh_cleanup.2

```tcl-visible
if {[namespace exists ::__nh_nh20261012cap2_namespace_ensemble]} {namespace delete ::__nh_nh20261012cap2_namespace_ensemble}; foreach nh_cmd [info commands ::__nh_nh20261012cap2_namespace_ensemble_*] {rename $nh_cmd {}}; list [namespace exists ::__nh_nh20261012cap2_namespace_ensemble] [info commands ::__nh_nh20261012cap2_namespace_ensemble_*]⟦EOF⟧
```

### nh_setup.3

```tcl-visible
namespace eval ::__nh_nh20261012cap2_namespace_upvar {variable x VALUE}⟦EOF⟧
```

### nh_operation.3

```tcl-visible
namespace upvar ::__nh_nh20261012cap2_namespace_upvar x nh_link; set nh_link⟦EOF⟧
```

### nh_cleanup.3

```tcl-visible
if {[namespace exists ::__nh_nh20261012cap2_namespace_upvar]} {namespace delete ::__nh_nh20261012cap2_namespace_upvar}; foreach nh_cmd [info commands ::__nh_nh20261012cap2_namespace_upvar_*] {rename $nh_cmd {}}; list [namespace exists ::__nh_nh20261012cap2_namespace_upvar] [info commands ::__nh_nh20261012cap2_namespace_upvar_*]⟦EOF⟧
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

- namespace.import, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- namespace.import, lookup, completion 0, result:

```text-visible
⟦EOF⟧
```

- namespace.import, runtime-cleanup, completion 0, result:

```text-visible
0 {}⟦EOF⟧
```

- namespace.ensemble, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, result:

```text-visible
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, errorinfo:

```text-visible
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace ensemble create -command facade -map {}"
    (in namespace eval "::__nh_nh20261012cap2_namespace_ensemble" script line 1)
    invoked from within
"namespace eval ::__nh_nh20261012cap2_namespace_ensemble {namespace ensemble create -command facade -map {}}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace.ensemble, runtime-cleanup, completion 0, result:

```text-visible
0 {}⟦EOF⟧
```

- namespace.upvar, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- namespace.upvar, lookup, completion 1, result:

```text-visible
bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace.upvar, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace.upvar, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- namespace.upvar, lookup, completion 1, errorinfo:

```text-visible
bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace upvar ::__nh_nh20261012cap2_namespace_upvar x nh_link"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace.upvar, runtime-cleanup, completion 0, result:

```text-visible
0 {}⟦EOF⟧
```

- namespace.ensemble: intended operation reached.

- namespace.import: intended operation reached.

- namespace.upvar: intended operation reached.

### tmm-procedure, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- namespace.import, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- namespace.import, lookup, completion 0, result:

```text-visible
⟦EOF⟧
```

- namespace.import, runtime-cleanup, completion 0, result:

```text-visible
0 {}⟦EOF⟧
```

- namespace.ensemble, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, result:

```text-visible
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, errorinfo:

```text-visible
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace ensemble create -command facade -map {}"
    (in namespace eval "::__nh_nh20261012cap2_namespace_ensemble" script line 1)
    invoked from within
"namespace eval ::__nh_nh20261012cap2_namespace_ensemble {namespace ensemble create -command facade -map {}}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace.ensemble, runtime-cleanup, completion 0, result:

```text-visible
0 {}⟦EOF⟧
```

- namespace.upvar, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- namespace.upvar, lookup, completion 1, result:

```text-visible
bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace.upvar, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace.upvar, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- namespace.upvar, lookup, completion 1, errorinfo:

```text-visible
bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace upvar ::__nh_nh20261012cap2_namespace_upvar x nh_link"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace.upvar, runtime-cleanup, completion 0, result:

```text-visible
0 {}⟦EOF⟧
```

- namespace.ensemble: intended operation reached.

- namespace.import: intended operation reached.

- namespace.upvar: intended operation reached.
