# BIG-IP Tcl log: nh20261012native1, tmm-RULE_INIT-9

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **owned-state-cleanup:** Which local/static::/qualified cells do rule-procedure calls read or write across caller and provider rules?

## Assumptions

- **owned-state-cleanup:** New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-rule-init-4](fixtures/bigip-tmm-rule-init-4.md)

- run: `nh20261012native1`
- case: `owned-state-cleanup`

### nh_operation

```tcl-visible
set n namespace
set owner 0
if {[info exists static::__nh_nh20261012native1_owner] && [set static::__nh_nh20261012native1_owner] == "nh20261012native1"} {set owner 1}
if {$owner} {if {[$n exists ::__nh_nh20261012native1]} {$n delete ::__nh_nh20261012native1}; foreach cell [list ::__nh_nh20261012native1_global static::__nh_nh20261012native1_static static::__nh_nh20261012native1_owner] {unset -nocomplain $cell}}
list owner $owner namespace [$n exists ::__nh_nh20261012native1] regular [info exists ::__nh_nh20261012native1_global] static [info exists static::__nh_nh20261012native1_static] marker [info exists static::__nh_nh20261012native1_owner]
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

- owned-state-cleanup, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- owned-state-cleanup, lookup, completion 0, result:

```text-visible
owner 1 namespace 0 regular 0 static 0 marker 0⟦EOF⟧
```

- owned-state-cleanup: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- owned-state-cleanup, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- owned-state-cleanup, lookup, completion 0, result:

```text-visible
owner 1 namespace 0 regular 0 static 0 marker 0⟦EOF⟧
```

- owned-state-cleanup: intended operation reached.
