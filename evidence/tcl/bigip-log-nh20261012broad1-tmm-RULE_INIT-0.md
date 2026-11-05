# BIG-IP Tcl log: nh20261012broad1, tmm-RULE_INIT-0

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **current-before-global:** Bare and relative-qualified command lookup: current, path, global order; qualifier namespace exists without target; no ancestor walk; absolute bypass. Does path exist?

- **namespace-path-relative:** Bare and relative-qualified command lookup: current, path, global order; qualifier namespace exists without target; no ancestor walk; absolute bypass. Does path exist?

- **relative-qualifier-missing-target:** Bare and relative-qualified command lookup: current, path, global order; qualifier namespace exists without target; no ancestor walk; absolute bypass. Does path exist?

## Assumptions

- **current-before-global:** NH01 adapted names; clean global command ::__tcl_lsp_probe_nh20261012broad15_current_before_global_p as well as ::__tcl_lsp_probe_nh20261012broad15_current_before_global.
- **namespace-path-relative:** NH03 adapted names; clean ::__tcl_lsp_probe_nh20261012broad15_namespace_path_relative_path too; path support is a separate control.
- **relative-qualifier-missing-target:** NH02 adapted names; clean ::__tcl_lsp_probe_nh20261012broad15_relative_qualifier_missing_target_target too; target qualifier exists locally.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-rule-init-5](fixtures/bigip-tmm-rule-init-5.md)

- run: `nh20261012broad1`
- case: `relative-qualifier-missing-target`
- case.2: `namespace-path-relative`
- case.3: `current-before-global`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_current_before_global {}
proc ::__tcl_lsp_probe_nh20261012broad15_current_before_global_p {} {return GLOBAL}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_current_before_global {proc __tcl_lsp_probe_nh20261012broad15_current_before_global_p {} {return CURRENT}}
```

### nh_operation

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_current_before_global {__tcl_lsp_probe_nh20261012broad15_current_before_global_p}
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad15_current_before_global* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_current_before_global*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad15_current_before_global*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad15_current_before_global*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_current_before_global*] [info commands ::__tcl_lsp_probe_nh20261012broad15_current_before_global*] [info vars ::__tcl_lsp_probe_nh20261012broad15_current_before_global*]⟦EOF⟧
```

### nh_setup.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_relative_qualifier_missing_target {}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_relative_qualifier_missing_target_target {proc p {} {return GLOBAL_TARGET}}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_relative_qualifier_missing_target {namespace eval __tcl_lsp_probe_nh20261012broad15_relative_qualifier_missing_target_target {}}
```

### nh_operation.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_relative_qualifier_missing_target {__tcl_lsp_probe_nh20261012broad15_relative_qualifier_missing_target_target::p}
```

### nh_cleanup.2

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad15_relative_qualifier_missing_target* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_relative_qualifier_missing_target*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad15_relative_qualifier_missing_target*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad15_relative_qualifier_missing_target*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_relative_qualifier_missing_target*] [info commands ::__tcl_lsp_probe_nh20261012broad15_relative_qualifier_missing_target*] [info vars ::__tcl_lsp_probe_nh20261012broad15_relative_qualifier_missing_target*]⟦EOF⟧
```

### nh_setup.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_namespace_path_relative {}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_namespace_path_relative_path::inner {proc p {} {return PATH_TARGET}}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_namespace_path_relative {namespace path ::__tcl_lsp_probe_nh20261012broad15_namespace_path_relative_path}
```

### nh_operation.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_namespace_path_relative {inner::p}
```

### nh_cleanup.3

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad15_namespace_path_relative* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_namespace_path_relative*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad15_namespace_path_relative*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad15_namespace_path_relative*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_namespace_path_relative*] [info commands ::__tcl_lsp_probe_nh20261012broad15_namespace_path_relative*] [info vars ::__tcl_lsp_probe_nh20261012broad15_namespace_path_relative*]⟦EOF⟧
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

- current-before-global, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- current-before-global, lookup, completion 0, result:

```text-visible
CURRENT⟦EOF⟧
```

- current-before-global, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- relative-qualifier-missing-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- relative-qualifier-missing-target, lookup, completion 0, result:

```text-visible
GLOBAL_TARGET⟦EOF⟧
```

- relative-qualifier-missing-target, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, result:

```text-visible
bad option "path": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, errorinfo:

```text-visible
bad option "path": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace path ::__tcl_lsp_probe_nh20261012broad15_namespace_path_relative_path"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_namespace_path_relative" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad15_namespace_path_relative {namespace path ::__tcl_lsp_probe_nh20261012broad15_namespace_path_relative..."
    ("eval" body line 3)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- namespace-path-relative, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- current-before-global: intended operation reached.

- namespace-path-relative: intended operation not reached.

- relative-qualifier-missing-target: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- current-before-global, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- current-before-global, lookup, completion 0, result:

```text-visible
CURRENT⟦EOF⟧
```

- current-before-global, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- relative-qualifier-missing-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- relative-qualifier-missing-target, lookup, completion 0, result:

```text-visible
GLOBAL_TARGET⟦EOF⟧
```

- relative-qualifier-missing-target, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, result:

```text-visible
bad option "path": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, errorinfo:

```text-visible
bad option "path": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace path ::__tcl_lsp_probe_nh20261012broad15_namespace_path_relative_path"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_namespace_path_relative" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad15_namespace_path_relative {namespace path ::__tcl_lsp_probe_nh20261012broad15_namespace_path_relative..."
    ("eval" body line 3)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- namespace-path-relative, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- current-before-global: intended operation reached.

- namespace-path-relative: intended operation not reached.

- relative-qualifier-missing-target: intended operation reached.
