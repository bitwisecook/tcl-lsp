# BIG-IP Tcl log: nh20261012broad1, tmm-RULE_INIT-17

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **error-link-array-element:** Error conformance: missing command/variable/namespace, forbidden link and rename/import conflict: result, completion, `-errorcode`, `-errorinfo`, options.

- **error-missing-namespace:** Error conformance: missing command/variable/namespace, forbidden link and rename/import conflict: result, completion, `-errorcode`, `-errorinfo`, options.

- **error-missing-variable:** Error conformance: missing command/variable/namespace, forbidden link and rename/import conflict: result, completion, `-errorcode`, `-errorinfo`, options.

## Assumptions

- **error-link-array-element:** Link targets an array element then attempts array access through scalar link.
- **error-missing-namespace:** Direct missing-namespace error after capability control.
- **error-missing-variable:** Direct missing-variable error, no inner catch.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-rule-init-3](fixtures/bigip-tmm-rule-init-3.md)

- run: `nh20261012broad1`
- case: `error-link-array-element`
- case.2: `error-missing-namespace`
- case.3: `error-missing-variable`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_error_missing_variable {}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_error_missing_variable {}
```

### nh_operation

```tcl-visible
set ::__tcl_lsp_probe_nh20261012broad15_error_missing_variable::absent
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad15_error_missing_variable* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_error_missing_variable*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad15_error_missing_variable*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad15_error_missing_variable*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_error_missing_variable*] [info commands ::__tcl_lsp_probe_nh20261012broad15_error_missing_variable*] [info vars ::__tcl_lsp_probe_nh20261012broad15_error_missing_variable*]⟦EOF⟧
```

### nh_setup.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_error_missing_namespace {}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_error_missing_namespace {}
```

### nh_operation.2

```tcl-visible
namespace inscope ::__tcl_lsp_probe_nh20261012broad15_error_missing_namespace::absent {set x}
```

### nh_cleanup.2

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad15_error_missing_namespace* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_error_missing_namespace*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad15_error_missing_namespace*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad15_error_missing_namespace*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_error_missing_namespace*] [info commands ::__tcl_lsp_probe_nh20261012broad15_error_missing_namespace*] [info vars ::__tcl_lsp_probe_nh20261012broad15_error_missing_namespace*]⟦EOF⟧
```

### nh_setup.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_error_link_array_element {}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_error_link_array_element {set arr(k) ELEMENT; proc target {} {upvar #0 ::__tcl_lsp_probe_nh20261012broad15_error_link_array_element::arr(k) link; set link(k) VALUE}}
```

### nh_operation.3

```tcl-visible
::__tcl_lsp_probe_nh20261012broad15_error_link_array_element::target
```

### nh_cleanup.3

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad15_error_link_array_element* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_error_link_array_element*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad15_error_link_array_element*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad15_error_link_array_element*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_error_link_array_element*] [info commands ::__tcl_lsp_probe_nh20261012broad15_error_link_array_element*] [info vars ::__tcl_lsp_probe_nh20261012broad15_error_link_array_element*]⟦EOF⟧
```

## Results

### tmm-RULE_INIT, unit 0:0, invocation once; tmm-RULE_INIT, unit 0:1, invocation once; tmm-RULE_INIT, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- error-missing-variable, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- error-missing-variable, lookup, completion 1, result:

```text-visible
can't read "::__tcl_lsp_probe_nh20261012broad15_error_missing_variable::absent": no such variable⟦EOF⟧
```

- error-missing-variable, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-missing-variable, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- error-missing-variable, lookup, completion 1, errorinfo:

```text-visible
can't read "::__tcl_lsp_probe_nh20261012broad15_error_missing_variable::absent": no such variable
    while executing
"set ::__tcl_lsp_probe_nh20261012broad15_error_missing_variable::absent"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-missing-variable, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- error-missing-namespace, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- error-missing-namespace, lookup, completion 1, result:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad15_error_missing_namespace::absent" in inscope namespace command⟦EOF⟧
```

- error-missing-namespace, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-missing-namespace, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- error-missing-namespace, lookup, completion 1, errorinfo:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad15_error_missing_namespace::absent" in inscope namespace command
    while executing
"namespace inscope ::__tcl_lsp_probe_nh20261012broad15_error_missing_namespace::absent {set x}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-missing-namespace, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- error-link-array-element, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- error-link-array-element, lookup, completion 1, result:

```text-visible
can't set "link(k)": variable isn't array⟦EOF⟧
```

- error-link-array-element, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-link-array-element, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- error-link-array-element, lookup, completion 1, errorinfo:

```text-visible
can't set "link(k)": variable isn't array
    while executing
"set link(k) VALUE"
    (procedure "::__tcl_lsp_probe_nh20261012broad15_error_link_array_element..." line 1)
    invoked from within
"::__tcl_lsp_probe_nh20261012broad15_error_link_array_element::target"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-link-array-element, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- error-link-array-element: intended operation reached.

- error-missing-namespace: intended operation reached.

- error-missing-variable: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- error-missing-variable, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- error-missing-variable, lookup, completion 1, result:

```text-visible
can't read "::__tcl_lsp_probe_nh20261012broad15_error_missing_variable::absent": no such variable⟦EOF⟧
```

- error-missing-variable, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-missing-variable, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- error-missing-variable, lookup, completion 1, errorinfo:

```text-visible
can't read "::__tcl_lsp_probe_nh20261012broad15_error_missing_variable::absent": no such variable
    while executing
"set ::__tcl_lsp_probe_nh20261012broad15_error_missing_variable::absent"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-missing-variable, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- error-missing-namespace, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- error-missing-namespace, lookup, completion 1, result:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad15_error_missing_namespace::absent" in inscope namespace command⟦EOF⟧
```

- error-missing-namespace, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-missing-namespace, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- error-missing-namespace, lookup, completion 1, errorinfo:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad15_error_missing_namespace::absent" in inscope namespace command
    while executing
"namespace inscope ::__tcl_lsp_probe_nh20261012broad15_error_missing_namespace::absent {set x}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-missing-namespace, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- error-link-array-element, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- error-link-array-element, lookup, completion 1, result:

```text-visible
can't set "link(k)": variable isn't array⟦EOF⟧
```

- error-link-array-element, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-link-array-element, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- error-link-array-element, lookup, completion 1, errorinfo:

```text-visible
can't set "link(k)": variable isn't array
    while executing
"set link(k) VALUE"
    (procedure "::__tcl_lsp_probe_nh20261012broad15_error_link_array_element..." line 1)
    invoked from within
"::__tcl_lsp_probe_nh20261012broad15_error_link_array_element::target"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-link-array-element, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- error-link-array-element: intended operation reached.

- error-missing-namespace: intended operation reached.

- error-missing-variable: intended operation reached.
