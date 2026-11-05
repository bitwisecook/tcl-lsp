# BIG-IP Tcl log: nh20261012broad2, tmm-procedure-0

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **error-import-conflict:** Error conformance: missing command/variable/namespace, forbidden link and rename/import conflict: result, completion, `-errorcode`, `-errorinfo`, options.

- **error-rename-conflict:** Error conformance: missing command/variable/namespace, forbidden link and rename/import conflict: result, completion, `-errorcode`, `-errorinfo`, options.

- **name-braced-head:** Do lexical accepted names and runtime evaluated names differ? Compare braced/quoted/substituted/list command prefixes, brace-newline rules and unaddressable-name output round trips.

## Assumptions

- **error-import-conflict:** Non-force import conflict between owned commands.
- **error-rename-conflict:** Conflict between owned commands only.
- **name-braced-head:** Braced spaced command spelling.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-procedure-6](fixtures/bigip-tmm-procedure-6.md)

- run: `nh20261012broad2`
- case: `error-rename-conflict`
- case.2: `error-import-conflict`
- case.3: `name-braced-head`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad26_error_rename_conflict {}
namespace eval ::__tcl_lsp_probe_nh20261012broad26_error_rename_conflict {proc source {} {return SOURCE}; proc destination {} {return DESTINATION}}
```

### nh_operation

```tcl-visible
rename ::__tcl_lsp_probe_nh20261012broad26_error_rename_conflict::source ::__tcl_lsp_probe_nh20261012broad26_error_rename_conflict::destination
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad26_error_rename_conflict* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad26_error_rename_conflict*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad26_error_rename_conflict*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad26_error_rename_conflict*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad26_error_rename_conflict*] [info commands ::__tcl_lsp_probe_nh20261012broad26_error_rename_conflict*] [info vars ::__tcl_lsp_probe_nh20261012broad26_error_rename_conflict*]⟦EOF⟧
```

### nh_setup.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad26_error_import_conflict {}
namespace eval ::__tcl_lsp_probe_nh20261012broad26_error_import_conflict::source {proc p {} {return SOURCE}; namespace export p}
namespace eval ::__tcl_lsp_probe_nh20261012broad26_error_import_conflict::consumer {proc p {} {return LOCAL}}
```

### nh_operation.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad26_error_import_conflict::consumer {namespace import ::__tcl_lsp_probe_nh20261012broad26_error_import_conflict::source::p}
```

### nh_cleanup.2

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad26_error_import_conflict* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad26_error_import_conflict*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad26_error_import_conflict*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad26_error_import_conflict*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad26_error_import_conflict*] [info commands ::__tcl_lsp_probe_nh20261012broad26_error_import_conflict*] [info vars ::__tcl_lsp_probe_nh20261012broad26_error_import_conflict*]⟦EOF⟧
```

### nh_setup.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad26_name_braced_head {}
namespace eval ::__tcl_lsp_probe_nh20261012broad26_name_braced_head {proc {two words} x {return $x}}
```

### nh_operation.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad26_name_braced_head {{two words} {a b}}
```

### nh_cleanup.3

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad26_name_braced_head* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad26_name_braced_head*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad26_name_braced_head*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad26_name_braced_head*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad26_name_braced_head*] [info commands ::__tcl_lsp_probe_nh20261012broad26_name_braced_head*] [info vars ::__tcl_lsp_probe_nh20261012broad26_name_braced_head*]⟦EOF⟧
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

- error-rename-conflict, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- error-rename-conflict, lookup, completion 1, result:

```text-visible
can't rename to "::__tcl_lsp_probe_nh20261012broad26_error_rename_conflict::destination": command already exists⟦EOF⟧
```

- error-rename-conflict, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-rename-conflict, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- error-rename-conflict, lookup, completion 1, errorinfo:

```text-visible
can't rename to "::__tcl_lsp_probe_nh20261012broad26_error_rename_conflict::destination": command already exists
    while executing
"rename ::__tcl_lsp_probe_nh20261012broad26_error_rename_conflict::source ::__tcl_lsp_probe_nh20261012broad26_error_rename_conflict::destination"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-rename-conflict, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- error-import-conflict, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, result:

```text-visible
can't import command "p": already exists⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, errorinfo:

```text-visible
can't import command "p": already exists
    while executing
"namespace import ::__tcl_lsp_probe_nh20261012broad26_error_import_conflict::source::p"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad26_error_import_conflict::consumer" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad26_error_import_conflict::consumer {namespace import ::__tcl_lsp_probe_nh20261012broad26_error_import_..."
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-import-conflict, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- name-braced-head, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- name-braced-head, lookup, completion 0, result:

```text-visible
a b⟦EOF⟧
```

- name-braced-head, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- error-import-conflict: intended operation reached.

- error-rename-conflict: intended operation reached.

- name-braced-head: intended operation reached.

### tmm-procedure, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- error-rename-conflict, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- error-rename-conflict, lookup, completion 1, result:

```text-visible
can't rename to "::__tcl_lsp_probe_nh20261012broad26_error_rename_conflict::destination": command already exists⟦EOF⟧
```

- error-rename-conflict, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-rename-conflict, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- error-rename-conflict, lookup, completion 1, errorinfo:

```text-visible
can't rename to "::__tcl_lsp_probe_nh20261012broad26_error_rename_conflict::destination": command already exists
    while executing
"rename ::__tcl_lsp_probe_nh20261012broad26_error_rename_conflict::source ::__tcl_lsp_probe_nh20261012broad26_error_rename_conflict::destination"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-rename-conflict, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- error-import-conflict, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, result:

```text-visible
can't import command "p": already exists⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, errorinfo:

```text-visible
can't import command "p": already exists
    while executing
"namespace import ::__tcl_lsp_probe_nh20261012broad26_error_import_conflict::source::p"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad26_error_import_conflict::consumer" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad26_error_import_conflict::consumer {namespace import ::__tcl_lsp_probe_nh20261012broad26_error_import_..."
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-import-conflict, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- name-braced-head, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- name-braced-head, lookup, completion 0, result:

```text-visible
a b⟦EOF⟧
```

- name-braced-head, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- error-import-conflict: intended operation reached.

- error-rename-conflict: intended operation reached.

- name-braced-head: intended operation reached.
