# BIG-IP Tcl log: nh20261012broad2, tmm-traffic-event-14

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **active-namespace-delete-recreate:** Namespace deletion/recreation while active: command/path/OO/trace tokens and caches; do old compiled descendants retain old identity?

- **namespace-code-recreated:** Namespace deletion/recreation while active: command/path/OO/trace tokens and caches; do old compiled descendants retain old identity?

- **protocol-relative-user-shadow:** Protocol prefix commands versus true namespaces: same-tail user shadows and profile/event availability, absolute/relative/computed invocation.

## Assumptions

- **active-namespace-delete-recreate:** Delete/recreate while proc frame is active; old and new cells observed in one call.
- **namespace-code-recreated:** Recreated namespace with same spelling.
- **protocol-relative-user-shadow:** User command shares protocol tail; native HTTP command is not used by setup.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-traffic-event-5](fixtures/bigip-tmm-traffic-event-5.md)

- run: `nh20261012broad2`
- case: `active-namespace-delete-recreate`
- case.2: `protocol-relative-user-shadow`
- case.3: `namespace-code-recreated`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_namespace_code_recreated {}
namespace eval ::__tcl_lsp_probe_nh20261012broad27_namespace_code_recreated::owner {set x OLD; set ::__tcl_lsp_probe_nh20261012broad27_namespace_code_recreated::prefix [namespace code {set x}]}
namespace delete ::__tcl_lsp_probe_nh20261012broad27_namespace_code_recreated::owner
namespace eval ::__tcl_lsp_probe_nh20261012broad27_namespace_code_recreated::owner {set x RECREATED}
```

### nh_operation

```tcl-visible
eval [set ::__tcl_lsp_probe_nh20261012broad27_namespace_code_recreated::prefix]
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad27_namespace_code_recreated* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_namespace_code_recreated*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad27_namespace_code_recreated*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad27_namespace_code_recreated*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_namespace_code_recreated*] [info commands ::__tcl_lsp_probe_nh20261012broad27_namespace_code_recreated*] [info vars ::__tcl_lsp_probe_nh20261012broad27_namespace_code_recreated*]⟦EOF⟧
```

### nh_setup.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_active_namespace_delete_recreate {}
namespace eval ::__tcl_lsp_probe_nh20261012broad27_active_namespace_delete_recreate::owner {variable x OLD; proc target {} {variable x; namespace delete ::__tcl_lsp_probe_nh20261012broad27_active_namespace_delete_recreate::owner; namespace eval ::__tcl_lsp_probe_nh20261012broad27_active_namespace_delete_recreate::owner {variable x NEW}; list $x [set ::__tcl_lsp_probe_nh20261012broad27_active_namespace_delete_recreate::owner::x]}}
```

### nh_operation.2

```tcl-visible
::__tcl_lsp_probe_nh20261012broad27_active_namespace_delete_recreate::owner::target
```

### nh_cleanup.2

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad27_active_namespace_delete_recreate* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_active_namespace_delete_recreate*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad27_active_namespace_delete_recreate*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad27_active_namespace_delete_recreate*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_active_namespace_delete_recreate*] [info commands ::__tcl_lsp_probe_nh20261012broad27_active_namespace_delete_recreate*] [info vars ::__tcl_lsp_probe_nh20261012broad27_active_namespace_delete_recreate*]⟦EOF⟧
```

### nh_setup.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_protocol_relative_user_shadow {}
namespace eval ::__tcl_lsp_probe_nh20261012broad27_protocol_relative_user_shadow::HTTP {proc method {} {return USER_SHADOW}}
```

### nh_operation.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_protocol_relative_user_shadow {HTTP::method}
```

### nh_cleanup.3

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad27_protocol_relative_user_shadow* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_protocol_relative_user_shadow*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad27_protocol_relative_user_shadow*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad27_protocol_relative_user_shadow*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_protocol_relative_user_shadow*] [info commands ::__tcl_lsp_probe_nh20261012broad27_protocol_relative_user_shadow*] [info vars ::__tcl_lsp_probe_nh20261012broad27_protocol_relative_user_shadow*]⟦EOF⟧
```

## Results

### tmm-traffic-event, unit 0:0, invocation 0; tmm-traffic-event, unit 0:0, invocation 1; tmm-traffic-event, unit 0:0, invocation 10; tmm-traffic-event, unit 0:0, invocation 11; tmm-traffic-event, unit 0:0, invocation 12; tmm-traffic-event, unit 0:0, invocation 13; tmm-traffic-event, unit 0:0, invocation 14; tmm-traffic-event, unit 0:0, invocation 15; tmm-traffic-event, unit 0:0, invocation 2; tmm-traffic-event, unit 0:0, invocation 3; tmm-traffic-event, unit 0:0, invocation 4; tmm-traffic-event, unit 0:0, invocation 5; tmm-traffic-event, unit 0:0, invocation 6; tmm-traffic-event, unit 0:0, invocation 7; tmm-traffic-event, unit 0:0, invocation 8

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- namespace-code-recreated, setup, completion 0, result:

```text-visible
RECREATED⟦EOF⟧
```

- namespace-code-recreated, lookup, completion 0, result:

```text-visible
RECREATED⟦EOF⟧
```

- namespace-code-recreated, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- active-namespace-delete-recreate, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- active-namespace-delete-recreate, lookup, completion 0, result:

```text-visible
OLD NEW⟦EOF⟧
```

- active-namespace-delete-recreate, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- protocol-relative-user-shadow, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- protocol-relative-user-shadow, lookup, completion 0, result:

```text-visible
USER_SHADOW⟦EOF⟧
```

- protocol-relative-user-shadow, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- active-namespace-delete-recreate: intended operation reached.

- namespace-code-recreated: intended operation reached.

- protocol-relative-user-shadow: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 9

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- namespace-code-recreated, setup, completion 0, result:

```text-visible
RECREATED⟦EOF⟧
```

- namespace-code-recreated, lookup, completion 0, result:

```text-visible
RECREATED⟦EOF⟧
```

- namespace-code-recreated, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- active-namespace-delete-recreate, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- active-namespace-delete-recreate, lookup, completion 0, result:

```text-visible
OLD NEW⟦EOF⟧
```

- active-namespace-delete-recreate, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- protocol-relative-user-shadow, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- protocol-relative-user-shadow, lookup, completion 0, result:

```text-visible
USER_SHADOW⟦EOF⟧
```

- protocol-relative-user-shadow, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- active-namespace-delete-recreate: intended operation reached.

- namespace-code-recreated: intended operation reached.

- protocol-relative-user-shadow: intended operation reached.
