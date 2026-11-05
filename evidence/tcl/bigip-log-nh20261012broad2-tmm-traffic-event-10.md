# BIG-IP Tcl log: nh20261012broad2, tmm-traffic-event-10

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **alias-rebound-target:** `interp alias` target namespace anchoring, late binding, qualified destination parent creation, describe/delete, child interpreters and duplicate prefixes.

- **alias-target-global-anchor:** `interp alias` target namespace anchoring, late binding, qualified destination parent creation, describe/delete, child interpreters and duplicate prefixes.

- **import-forget-global-origin:** Import/export/force/forget/origin support and lifecycle; global-origin imports, source rename/redefinition/deletion, chains and conflicts.

## Assumptions

- **alias-rebound-target:** NH13 adapted owned namespace; before call is a setup observation.
- **alias-target-global-anchor:** Alias target uses bare unique tail; clean global command ::__tcl_lsp_probe_nh20261012broad27_alias_target_global_anchor_target.
- **import-forget-global-origin:** NH16 adapted names; fresh interpreter only, global export state changes; clean ::__tcl_lsp_probe_nh20261012broad27_import_forget_global_origin_source. Child-interpreter adaptation.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-traffic-event-9](fixtures/bigip-tmm-traffic-event-9.md)

- case: `import-forget-global-origin`
- case.2: `alias-target-global-anchor`
- case.3: `alias-rebound-target`

### nh_setup

```tcl-visible
set nh_fresh [interp create]⟦EOF⟧
```

### nh_operation

```tcl-visible
interp eval $nh_fresh [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f666f726765745f676c6f62616c5f6f726967696e207b7d0a70726f63203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f666f726765745f676c6f62616c5f6f726967696e5f736f75726365207b7d207b72657475726e20474c4f42414c5f494d504f52547d0a6e616d657370616365206576616c203a3a207b6e616d657370616365206578706f7274205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f666f726765745f676c6f62616c5f6f726967696e5f736f757263657d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f666f726765745f676c6f62616c5f6f726967696e207b6e616d65737061636520696d706f7274203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f666f726765745f676c6f62616c5f6f726967696e5f736f757263653b20736574206265666f7265205b5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f666f726765745f676c6f62616c5f6f726967696e5f736f757263655d7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f666f726765745f676c6f62616c5f6f726967696e207b6e616d65737061636520666f72676574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f666f726765745f676c6f62616c5f6f726967696e5f736f757263653b206c69737420246265666f7265205b6e616d657370616365207768696368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f666f726765745f676c6f62616c5f6f726967696e5f736f757263655d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f666f726765745f676c6f62616c5f6f726967696e3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f666f726765745f676c6f62616c5f6f726967696e5f736f757263655d7d0a]⟦EOF⟧
```

### nh_cleanup

```tcl-visible
interp delete $nh_fresh⟦EOF⟧
```

### nh_setup.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_alias_rebound_target {}
namespace eval ::__tcl_lsp_probe_nh20261012broad27_alias_rebound_target {proc target {} {return OLD}}
interp alias {} ::__tcl_lsp_probe_nh20261012broad27_alias_rebound_target::alias {} ::__tcl_lsp_probe_nh20261012broad27_alias_rebound_target::target
set ::__tcl_lsp_probe_nh20261012broad27_alias_rebound_target::before [::__tcl_lsp_probe_nh20261012broad27_alias_rebound_target::alias]
proc ::__tcl_lsp_probe_nh20261012broad27_alias_rebound_target::target {} {return NEW}
```

### nh_operation.2

```tcl-visible
list [set ::__tcl_lsp_probe_nh20261012broad27_alias_rebound_target::before] [::__tcl_lsp_probe_nh20261012broad27_alias_rebound_target::alias]
```

### nh_cleanup.2

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad27_alias_rebound_target* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_alias_rebound_target*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad27_alias_rebound_target*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad27_alias_rebound_target*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_alias_rebound_target*] [info commands ::__tcl_lsp_probe_nh20261012broad27_alias_rebound_target*] [info vars ::__tcl_lsp_probe_nh20261012broad27_alias_rebound_target*]⟦EOF⟧
```

### nh_setup.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_alias_target_global_anchor {}
proc ::__tcl_lsp_probe_nh20261012broad27_alias_target_global_anchor_target {} {return GLOBAL}
namespace eval ::__tcl_lsp_probe_nh20261012broad27_alias_target_global_anchor {proc __tcl_lsp_probe_nh20261012broad27_alias_target_global_anchor_target {} {return LOCAL}; interp alias {} ::__tcl_lsp_probe_nh20261012broad27_alias_target_global_anchor::alias {} __tcl_lsp_probe_nh20261012broad27_alias_target_global_anchor_target}
```

### nh_operation.3

```tcl-visible
::__tcl_lsp_probe_nh20261012broad27_alias_target_global_anchor::alias
```

### nh_cleanup.3

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad27_alias_target_global_anchor* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_alias_target_global_anchor*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad27_alias_target_global_anchor*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad27_alias_target_global_anchor*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_alias_target_global_anchor*] [info commands ::__tcl_lsp_probe_nh20261012broad27_alias_target_global_anchor*] [info vars ::__tcl_lsp_probe_nh20261012broad27_alias_target_global_anchor*]⟦EOF⟧
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

- import-forget-global-origin, setup, completion 0, result:

```text-visible
interp0⟦EOF⟧
```

- import-forget-global-origin, lookup, completion 0, result:

```text-visible
GLOBAL_IMPORT ::__tcl_lsp_probe_nh20261012broad27_import_forget_global_origin_source {}⟦EOF⟧
```

- import-forget-global-origin, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- alias-rebound-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- alias-rebound-target, lookup, completion 0, result:

```text-visible
OLD NEW⟦EOF⟧
```

- alias-rebound-target, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- alias-target-global-anchor, setup, completion 0, result:

```text-visible
::__tcl_lsp_probe_nh20261012broad27_alias_target_global_anchor::alias⟦EOF⟧
```

- alias-target-global-anchor, lookup, completion 0, result:

```text-visible
GLOBAL⟦EOF⟧
```

- alias-target-global-anchor, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- alias-rebound-target: intended operation reached.

- alias-target-global-anchor: intended operation reached.

- import-forget-global-origin: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 9

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- import-forget-global-origin, setup, completion 0, result:

```text-visible
interp0⟦EOF⟧
```

- import-forget-global-origin, lookup, completion 0, result:

```text-visible
GLOBAL_IMPORT ::__tcl_lsp_probe_nh20261012broad27_import_forget_global_origin_source {}⟦EOF⟧
```

- import-forget-global-origin, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- alias-rebound-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- alias-rebound-target, lookup, completion 0, result:

```text-visible
OLD NEW⟦EOF⟧
```

- alias-rebound-target, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- alias-target-global-anchor, setup, completion 0, result:

```text-visible
::__tcl_lsp_probe_nh20261012broad27_alias_target_global_anchor::alias⟦EOF⟧
```

- alias-target-global-anchor, lookup, completion 0, result:

```text-visible
GLOBAL⟦EOF⟧
```

- alias-target-global-anchor, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- alias-rebound-target: intended operation reached.

- alias-target-global-anchor: intended operation reached.

- import-forget-global-origin: intended operation reached.
