# BIG-IP Tcl log: nh20261012broad1, tmm-procedure-4

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **literal-variable-name-data:** Literal `{}`, `{$n}`, `{{a}}`, Unicode and byte-valued names; array indices with colons/parentheses. Are byte values representable in each context?

- **namespace-exists-global-fallback:** Qualified/relative-qualified variables and namespace-scope missing-read fallback: read/write/create/exists differences.

- **relative-variable-missing-parent:** Qualified/relative-qualified variables and namespace-scope missing-read fallback: read/write/create/exists differences.

## Assumptions

- **literal-variable-name-data:** NH08 adapted namespace; braced spellings remain literal.
- **namespace-exists-global-fallback:** Existence contrast to missing read; clean ::__tcl_lsp_probe_nh20261012broad16_namespace_exists_global_fallback_x.
- **relative-variable-missing-parent:** Creation with nonexistent relative qualifier; expected error is a measurement.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-procedure-5](fixtures/bigip-tmm-procedure-5.md)

- case: `namespace-exists-global-fallback`
- case.2: `relative-variable-missing-parent`
- case.3: `literal-variable-name-data`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad16_namespace_exists_global_fallback {}
set ::__tcl_lsp_probe_nh20261012broad16_namespace_exists_global_fallback_x GLOBAL_VALUE
namespace eval ::__tcl_lsp_probe_nh20261012broad16_namespace_exists_global_fallback {}
```

### nh_operation

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad16_namespace_exists_global_fallback {info exists __tcl_lsp_probe_nh20261012broad16_namespace_exists_global_fallback_x}
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad16_namespace_exists_global_fallback* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad16_namespace_exists_global_fallback*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad16_namespace_exists_global_fallback*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad16_namespace_exists_global_fallback*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad16_namespace_exists_global_fallback*] [info commands ::__tcl_lsp_probe_nh20261012broad16_namespace_exists_global_fallback*] [info vars ::__tcl_lsp_probe_nh20261012broad16_namespace_exists_global_fallback*]⟦EOF⟧
```

### nh_setup.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad16_relative_variable_missing_parent {}
namespace eval ::__tcl_lsp_probe_nh20261012broad16_relative_variable_missing_parent {}
```

### nh_operation.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad16_relative_variable_missing_parent {set absent::x VALUE}
```

### nh_cleanup.2

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad16_relative_variable_missing_parent* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad16_relative_variable_missing_parent*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad16_relative_variable_missing_parent*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad16_relative_variable_missing_parent*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad16_relative_variable_missing_parent*] [info commands ::__tcl_lsp_probe_nh20261012broad16_relative_variable_missing_parent*] [info vars ::__tcl_lsp_probe_nh20261012broad16_relative_variable_missing_parent*]⟦EOF⟧
```

### nh_setup.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad16_literal_variable_name_data {}
namespace eval ::__tcl_lsp_probe_nh20261012broad16_literal_variable_name_data {set {} EMPTY; set {$n} LITERAL_DOLLAR; set {{a}} LITERAL_BRACES}
```

### nh_operation.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad16_literal_variable_name_data {list [set {}] [set {$n}] [set {{a}}] [info exists {{a}}]}
```

### nh_cleanup.3

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad16_literal_variable_name_data* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad16_literal_variable_name_data*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad16_literal_variable_name_data*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad16_literal_variable_name_data*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad16_literal_variable_name_data*] [info commands ::__tcl_lsp_probe_nh20261012broad16_literal_variable_name_data*] [info vars ::__tcl_lsp_probe_nh20261012broad16_literal_variable_name_data*]⟦EOF⟧
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

- namespace-exists-global-fallback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- namespace-exists-global-fallback, lookup, completion 0, result:

```text-visible
1⟦EOF⟧
```

- namespace-exists-global-fallback, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- relative-variable-missing-parent, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, result:

```text-visible
can't set "absent::x": parent namespace doesn't exist⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, errorinfo:

```text-visible
can't set "absent::x": parent namespace doesn't exist
    while executing
"set absent::x VALUE"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad16_relative_variable_missing_parent" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad16_relative_variable_missing_parent {set absent::x VALUE}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- relative-variable-missing-parent, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- literal-variable-name-data, setup, completion 0, result:

```text-visible
LITERAL_BRACES⟦EOF⟧
```

- literal-variable-name-data, lookup, completion 0, result:

```text-visible
EMPTY LITERAL_DOLLAR LITERAL_BRACES 1⟦EOF⟧
```

- literal-variable-name-data, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- literal-variable-name-data: intended operation reached.

- namespace-exists-global-fallback: intended operation reached.

- relative-variable-missing-parent: intended operation reached.

### tmm-procedure, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- namespace-exists-global-fallback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- namespace-exists-global-fallback, lookup, completion 0, result:

```text-visible
1⟦EOF⟧
```

- namespace-exists-global-fallback, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- relative-variable-missing-parent, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, result:

```text-visible
can't set "absent::x": parent namespace doesn't exist⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, errorinfo:

```text-visible
can't set "absent::x": parent namespace doesn't exist
    while executing
"set absent::x VALUE"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad16_relative_variable_missing_parent" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad16_relative_variable_missing_parent {set absent::x VALUE}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- relative-variable-missing-parent, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- literal-variable-name-data, setup, completion 0, result:

```text-visible
LITERAL_BRACES⟦EOF⟧
```

- literal-variable-name-data, lookup, completion 0, result:

```text-visible
EMPTY LITERAL_DOLLAR LITERAL_BRACES 1⟦EOF⟧
```

- literal-variable-name-data, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- literal-variable-name-data: intended operation reached.

- namespace-exists-global-fallback: intended operation reached.

- relative-variable-missing-parent: intended operation reached.
