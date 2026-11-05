# BIG-IP Tcl log: nh20261012broad1, tmm-RULE_INIT-11

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **alias-child-target-owner:** `interp alias` target namespace anchoring, late binding, qualified destination parent creation, describe/delete, child interpreters and duplicate prefixes.

- **alias-describe-prefix:** `interp alias` target namespace anchoring, late binding, qualified destination parent creation, describe/delete, child interpreters and duplicate prefixes.

- **alias-destination-missing-parent:** `interp alias` target namespace anchoring, late binding, qualified destination parent creation, describe/delete, child interpreters and duplicate prefixes.

## Assumptions

- **alias-child-target-owner:** Child-owned target; delete child interpreter __tcl_lsp_probe_nh20261012broad15_alias_child_target_owner_child before namespace teardown.
- **alias-describe-prefix:** Description retains duplicated/space-bearing prefix words without flattening.
- **alias-destination-missing-parent:** Observe whether qualified alias destination creates missing parent.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-rule-init-5](fixtures/bigip-tmm-rule-init-5.md)

- run: `nh20261012broad1`
- case: `alias-destination-missing-parent`
- case.2: `alias-child-target-owner`
- case.3: `alias-describe-prefix`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_alias_describe_prefix {}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_alias_describe_prefix {proc target args {return $args}}
interp alias {} ::__tcl_lsp_probe_nh20261012broad15_alias_describe_prefix::alias {} ::__tcl_lsp_probe_nh20261012broad15_alias_describe_prefix::target {a b} {c d}
```

### nh_operation

```tcl-visible
interp alias {} ::__tcl_lsp_probe_nh20261012broad15_alias_describe_prefix::alias
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad15_alias_describe_prefix* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_alias_describe_prefix*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad15_alias_describe_prefix*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad15_alias_describe_prefix*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_alias_describe_prefix*] [info commands ::__tcl_lsp_probe_nh20261012broad15_alias_describe_prefix*] [info vars ::__tcl_lsp_probe_nh20261012broad15_alias_describe_prefix*]⟦EOF⟧
```

### nh_setup.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_alias_destination_missing_parent {}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_alias_destination_missing_parent {proc target {} {return TARGET}}
```

### nh_operation.2

```tcl-visible
interp alias {} ::__tcl_lsp_probe_nh20261012broad15_alias_destination_missing_parent::absent::alias {} ::__tcl_lsp_probe_nh20261012broad15_alias_destination_missing_parent::target
```

### nh_cleanup.2

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad15_alias_destination_missing_parent* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_alias_destination_missing_parent*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad15_alias_destination_missing_parent*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad15_alias_destination_missing_parent*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_alias_destination_missing_parent*] [info commands ::__tcl_lsp_probe_nh20261012broad15_alias_destination_missing_parent*] [info vars ::__tcl_lsp_probe_nh20261012broad15_alias_destination_missing_parent*]⟦EOF⟧
```

### nh_setup.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_alias_child_target_owner {}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_alias_child_target_owner {proc target {} {return MAIN}}
interp create __tcl_lsp_probe_nh20261012broad15_alias_child_target_owner_child
interp eval __tcl_lsp_probe_nh20261012broad15_alias_child_target_owner_child {proc target {} {return CHILD}}
interp alias {} ::__tcl_lsp_probe_nh20261012broad15_alias_child_target_owner::alias __tcl_lsp_probe_nh20261012broad15_alias_child_target_owner_child target
```

### nh_operation.3

```tcl-visible
::__tcl_lsp_probe_nh20261012broad15_alias_child_target_owner::alias
```

### nh_cleanup.3

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad15_alias_child_target_owner* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_alias_child_target_owner*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad15_alias_child_target_owner*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad15_alias_child_target_owner*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_alias_child_target_owner*] [info commands ::__tcl_lsp_probe_nh20261012broad15_alias_child_target_owner*] [info vars ::__tcl_lsp_probe_nh20261012broad15_alias_child_target_owner*]⟦EOF⟧
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

- alias-describe-prefix, setup, completion 0, result:

```text-visible
::__tcl_lsp_probe_nh20261012broad15_alias_describe_prefix::alias⟦EOF⟧
```

- alias-describe-prefix, lookup, completion 0, result:

```text-visible
::__tcl_lsp_probe_nh20261012broad15_alias_describe_prefix::target {a b} {c d}⟦EOF⟧
```

- alias-describe-prefix, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- alias-destination-missing-parent, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- alias-destination-missing-parent, lookup, completion 0, result:

```text-visible
::__tcl_lsp_probe_nh20261012broad15_alias_destination_missing_parent::absent::alias⟦EOF⟧
```

- alias-destination-missing-parent, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- alias-child-target-owner, setup, completion 0, result:

```text-visible
::__tcl_lsp_probe_nh20261012broad15_alias_child_target_owner::alias⟦EOF⟧
```

- alias-child-target-owner, lookup, completion 0, result:

```text-visible
CHILD⟦EOF⟧
```

- alias-child-target-owner, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- alias-child-target-owner: intended operation reached.

- alias-describe-prefix: intended operation reached.

- alias-destination-missing-parent: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-describe-prefix, setup, completion 0, result:

```text-visible
::__tcl_lsp_probe_nh20261012broad15_alias_describe_prefix::alias⟦EOF⟧
```

- alias-describe-prefix, lookup, completion 0, result:

```text-visible
::__tcl_lsp_probe_nh20261012broad15_alias_describe_prefix::target {a b} {c d}⟦EOF⟧
```

- alias-describe-prefix, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- alias-destination-missing-parent, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- alias-destination-missing-parent, lookup, completion 0, result:

```text-visible
::__tcl_lsp_probe_nh20261012broad15_alias_destination_missing_parent::absent::alias⟦EOF⟧
```

- alias-destination-missing-parent, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- alias-child-target-owner, setup, completion 0, result:

```text-visible
::__tcl_lsp_probe_nh20261012broad15_alias_child_target_owner::alias⟦EOF⟧
```

- alias-child-target-owner, lookup, completion 0, result:

```text-visible
CHILD⟦EOF⟧
```

- alias-child-target-owner, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- alias-child-target-owner: intended operation reached.

- alias-describe-prefix: intended operation reached.

- alias-destination-missing-parent: intended operation reached.
