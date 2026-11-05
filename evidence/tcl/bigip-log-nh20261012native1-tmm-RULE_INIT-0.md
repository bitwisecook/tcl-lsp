# BIG-IP Tcl log: nh20261012native1, tmm-RULE_INIT-0

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **owned-native-state-absence:** Which local/static::/qualified cells do rule-procedure calls read or write across caller and provider rules?

## Assumptions

- **owned-native-state-absence:** New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-rule-init-4](fixtures/bigip-tmm-rule-init-4.md)

- run: `nh20261012native1`
- case: `owned-native-state-absence`

### nh_operation

```tcl-visible
set n namespace
if {[$n exists ::__nh_nh20261012native1]} {error {Owned namespace collision}}
foreach cell [list ::__nh_nh20261012native1_global static::__nh_nh20261012native1_static static::__nh_nh20261012native1_owner] {if {[info exists $cell] || [llength [trace vinfo $cell]]} {error [list Owned-cell-collision $cell]}}
foreach target [list /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target /Common/__tcl_lsp_probe_nh20261012native1_other::__nh_nh20261012native1_target /Common/__tcl_lsp_probe_nh20261012native1_folder/__tcl_lsp_probe_nh20261012native1_folded::__nh_nh20261012native1_target /NH_nh20261012native1/__tcl_lsp_probe_nh20261012native1_foreign::__nh_nh20261012native1_target] {
 set c [catch {eval [list call $target __guard__ __guard__]} v]
 if {$c != 1 || ![string match {*not found*} $v]} {error [list Native-absence-not-proved $target $c $v]}
}
list absent ::__nh_nh20261012native1 ::__nh_nh20261012native1_global static::__nh_nh20261012native1_static static::__nh_nh20261012native1_owner /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target /Common/__tcl_lsp_probe_nh20261012native1_other::__nh_nh20261012native1_target /Common/__tcl_lsp_probe_nh20261012native1_folder/__tcl_lsp_probe_nh20261012native1_folded::__nh_nh20261012native1_target /NH_nh20261012native1/__tcl_lsp_probe_nh20261012native1_foreign::__nh_nh20261012native1_target
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

- owned-native-state-absence, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- owned-native-state-absence, lookup, completion 0, result:

```text-visible
absent ::__nh_nh20261012native1 ::__nh_nh20261012native1_global static::__nh_nh20261012native1_static static::__nh_nh20261012native1_owner /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target /Common/__tcl_lsp_probe_nh20261012native1_other::__nh_nh20261012native1_target /Common/__tcl_lsp_probe_nh20261012native1_folder/__tcl_lsp_probe_nh20261012native1_folded::__nh_nh20261012native1_target /NH_nh20261012native1/__tcl_lsp_probe_nh20261012native1_foreign::__nh_nh20261012native1_target⟦EOF⟧
```

- owned-native-state-absence: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- owned-native-state-absence, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- owned-native-state-absence, lookup, completion 0, result:

```text-visible
absent ::__nh_nh20261012native1 ::__nh_nh20261012native1_global static::__nh_nh20261012native1_static static::__nh_nh20261012native1_owner /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target /Common/__tcl_lsp_probe_nh20261012native1_other::__nh_nh20261012native1_target /Common/__tcl_lsp_probe_nh20261012native1_folder/__tcl_lsp_probe_nh20261012native1_folded::__nh_nh20261012native1_target /NH_nh20261012native1/__tcl_lsp_probe_nh20261012native1_foreign::__nh_nh20261012native1_target⟦EOF⟧
```

- owned-native-state-absence: intended operation reached.
