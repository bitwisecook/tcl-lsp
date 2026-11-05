# BIG-IP Tcl log: nh20261012persist1, tmm-RULE_INIT-0

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **worker-owned-absence:** Persistence and isolation: definitions/variables from RULE_INIT into proc/events and repeated requests, per TMM/per connection/cross-rule scope.

## Assumptions

- **worker-owned-absence:** Exact newly authored owned control; native F5 provider and actual host frame. Nested completion codes/results are Tcl list data; unavailable inner error options are not inferred.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-rule-init-4](fixtures/bigip-tmm-rule-init-4.md)

- run: `nh20261012persist1`
- case: `worker-owned-absence`

### nh_operation

```tcl-visible
set n namespace
if {[$n exists ::__nh_nh20261012persist1]} {error {Owned namespace collision}}
foreach cell [list ::__nh_nh20261012persist1_global static::__nh_nh20261012persist1_static static::__nh_nh20261012persist1_owner] {
 if {[info exists $cell] || [llength [info vars $cell]] || [llength [trace vinfo $cell]]} {error [list Owned-cell-collision $cell]}
}
foreach owned [list ::__nh_nh20261012persist1_dynamic] {if {[llength [info commands $owned]]} {error [list Owned-dynamic-collision $owned]}}
foreach head [list __nh_nh20261012persist1_target /Common/__tcl_lsp_probe_nh20261012persist1_provider::__nh_nh20261012persist1_target /Common/__tcl_lsp_probe_nh20261012persist1_provider::__nh_nh20261012persist1_target_moved] {if {[llength [info commands $head]]} {error [list Owned-command-collision $head]}}
list absent ::__nh_nh20261012persist1 ::__nh_nh20261012persist1_global static::__nh_nh20261012persist1_static static::__nh_nh20261012persist1_owner __nh_nh20261012persist1_target /Common/__tcl_lsp_probe_nh20261012persist1_provider::__nh_nh20261012persist1_target /Common/__tcl_lsp_probe_nh20261012persist1_provider::__nh_nh20261012persist1_target_moved
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

- worker-owned-absence, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- worker-owned-absence, lookup, completion 0, result:

```text-visible
absent ::__nh_nh20261012persist1 ::__nh_nh20261012persist1_global static::__nh_nh20261012persist1_static static::__nh_nh20261012persist1_owner __nh_nh20261012persist1_target /Common/__tcl_lsp_probe_nh20261012persist1_provider::__nh_nh20261012persist1_target /Common/__tcl_lsp_probe_nh20261012persist1_provider::__nh_nh20261012persist1_target_moved⟦EOF⟧
```

- worker-owned-absence: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- worker-owned-absence, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- worker-owned-absence, lookup, completion 0, result:

```text-visible
absent ::__nh_nh20261012persist1 ::__nh_nh20261012persist1_global static::__nh_nh20261012persist1_static static::__nh_nh20261012persist1_owner __nh_nh20261012persist1_target /Common/__tcl_lsp_probe_nh20261012persist1_provider::__nh_nh20261012persist1_target /Common/__tcl_lsp_probe_nh20261012persist1_provider::__nh_nh20261012persist1_target_moved⟦EOF⟧
```

- worker-owned-absence: intended operation reached.
