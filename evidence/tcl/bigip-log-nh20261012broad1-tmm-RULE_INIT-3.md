# BIG-IP Tcl log: nh20261012broad1, tmm-RULE_INIT-3

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **namespace-missing-read-global-fallback:** Qualified/relative-qualified variables and namespace-scope missing-read fallback: read/write/create/exists differences.

- **namespace-missing-write-local:** Qualified/relative-qualified variables and namespace-scope missing-read fallback: read/write/create/exists differences.

- **relative-variable-write-read:** Qualified/relative-qualified variables and namespace-scope missing-read fallback: read/write/create/exists differences.

## Assumptions

- **namespace-missing-read-global-fallback:** NH06 adapted names; clean global variable ::__tcl_lsp_probe_nh20261012broad15_namespace_missing_read_global_fallback_x.
- **namespace-missing-write-local:** Namespace write/create contrast; clean global variable ::__tcl_lsp_probe_nh20261012broad15_namespace_missing_write_local_x.
- **relative-variable-write-read:** NH05 adapted names.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-rule-init-5](fixtures/bigip-tmm-rule-init-5.md)

- run: `nh20261012broad1`
- case: `namespace-missing-read-global-fallback`
- case.2: `namespace-missing-write-local`
- case.3: `relative-variable-write-read`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_relative_variable_write_read {}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_relative_variable_write_read::n {variable x 0}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_relative_variable_write_read {proc run {} {set n::x 7; return $n::x}}
```

### nh_operation

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_relative_variable_write_read {list [run] [set n::x]}
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad15_relative_variable_write_read* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_relative_variable_write_read*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad15_relative_variable_write_read*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad15_relative_variable_write_read*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_relative_variable_write_read*] [info commands ::__tcl_lsp_probe_nh20261012broad15_relative_variable_write_read*] [info vars ::__tcl_lsp_probe_nh20261012broad15_relative_variable_write_read*]⟦EOF⟧
```

### nh_setup.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_namespace_missing_read_global_fallback {}
set ::__tcl_lsp_probe_nh20261012broad15_namespace_missing_read_global_fallback_x GLOBAL_VALUE
namespace eval ::__tcl_lsp_probe_nh20261012broad15_namespace_missing_read_global_fallback {}
```

### nh_operation.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_namespace_missing_read_global_fallback {set __tcl_lsp_probe_nh20261012broad15_namespace_missing_read_global_fallback_x}
```

### nh_cleanup.2

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad15_namespace_missing_read_global_fallback* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_namespace_missing_read_global_fallback*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad15_namespace_missing_read_global_fallback*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad15_namespace_missing_read_global_fallback*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_namespace_missing_read_global_fallback*] [info commands ::__tcl_lsp_probe_nh20261012broad15_namespace_missing_read_global_fallback*] [info vars ::__tcl_lsp_probe_nh20261012broad15_namespace_missing_read_global_fallback*]⟦EOF⟧
```

### nh_setup.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_namespace_missing_write_local {}
set ::__tcl_lsp_probe_nh20261012broad15_namespace_missing_write_local_x GLOBAL_VALUE
namespace eval ::__tcl_lsp_probe_nh20261012broad15_namespace_missing_write_local {}
```

### nh_operation.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_namespace_missing_write_local {set __tcl_lsp_probe_nh20261012broad15_namespace_missing_write_local_x LOCAL_VALUE}; list [set ::__tcl_lsp_probe_nh20261012broad15_namespace_missing_write_local_x] [set ::__tcl_lsp_probe_nh20261012broad15_namespace_missing_write_local::__tcl_lsp_probe_nh20261012broad15_namespace_missing_write_local_x]
```

### nh_cleanup.3

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad15_namespace_missing_write_local* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_namespace_missing_write_local*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad15_namespace_missing_write_local*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad15_namespace_missing_write_local*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_namespace_missing_write_local*] [info commands ::__tcl_lsp_probe_nh20261012broad15_namespace_missing_write_local*] [info vars ::__tcl_lsp_probe_nh20261012broad15_namespace_missing_write_local*]⟦EOF⟧
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

- relative-variable-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- relative-variable-write-read, lookup, completion 0, result:

```text-visible
7 7⟦EOF⟧
```

- relative-variable-write-read, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-missing-read-global-fallback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- namespace-missing-read-global-fallback, lookup, completion 0, result:

```text-visible
GLOBAL_VALUE⟦EOF⟧
```

- namespace-missing-read-global-fallback, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-missing-write-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- namespace-missing-write-local, lookup, completion 1, result:

```text-visible
can't read "::__tcl_lsp_probe_nh20261012broad15_namespace_missing_write_local::__tcl_lsp_probe_nh20261012broad15_namespace_missing_write_local_x": no such variable⟦EOF⟧
```

- namespace-missing-write-local, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-missing-write-local, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- namespace-missing-write-local, lookup, completion 1, errorinfo:

```text-visible
can't read "::__tcl_lsp_probe_nh20261012broad15_namespace_missing_write_local::__tcl_lsp_probe_nh20261012broad15_namespace_missing_write_local_x": no such variable
    while executing
"set ::__tcl_lsp_probe_nh20261012broad15_namespace_missing_write_local::__tcl_lsp_probe_nh20261012broad15_namespace_missing_write_local_x"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace-missing-write-local, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-missing-read-global-fallback: intended operation reached.

- namespace-missing-write-local: intended operation reached.

- relative-variable-write-read: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- relative-variable-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- relative-variable-write-read, lookup, completion 0, result:

```text-visible
7 7⟦EOF⟧
```

- relative-variable-write-read, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-missing-read-global-fallback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- namespace-missing-read-global-fallback, lookup, completion 0, result:

```text-visible
GLOBAL_VALUE⟦EOF⟧
```

- namespace-missing-read-global-fallback, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-missing-write-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- namespace-missing-write-local, lookup, completion 1, result:

```text-visible
can't read "::__tcl_lsp_probe_nh20261012broad15_namespace_missing_write_local::__tcl_lsp_probe_nh20261012broad15_namespace_missing_write_local_x": no such variable⟦EOF⟧
```

- namespace-missing-write-local, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-missing-write-local, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- namespace-missing-write-local, lookup, completion 1, errorinfo:

```text-visible
can't read "::__tcl_lsp_probe_nh20261012broad15_namespace_missing_write_local::__tcl_lsp_probe_nh20261012broad15_namespace_missing_write_local_x": no such variable
    while executing
"set ::__tcl_lsp_probe_nh20261012broad15_namespace_missing_write_local::__tcl_lsp_probe_nh20261012broad15_namespace_missing_write_local_x"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace-missing-write-local, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-missing-read-global-fallback: intended operation reached.

- namespace-missing-write-local: intended operation reached.

- relative-variable-write-read: intended operation reached.
