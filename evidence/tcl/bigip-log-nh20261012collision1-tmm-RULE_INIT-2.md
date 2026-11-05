# BIG-IP Tcl log: nh20261012collision1, tmm-RULE_INIT-2

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **alias-prefix-lifetime:** After an interpreter alias with a fixed multiword argument is renamed, what does querying the old alias name return, and how does the renamed command apply its retained prefix to a new argument?

## Assumptions

- **alias-prefix-lifetime:** Complete separately adapted program in actual host. Intended original final lookup is not inferred from whole-program completion. Enumerated spelling substitutions avoid unowned native commands/frame variables.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-rule-init-1](fixtures/bigip-tmm-rule-init-1.md)

- run: `nh20261012collision1`
- case: `alias-prefix-lifetime`

### nh_setup

```tcl-visible
if {[llength [info commands ::emit]]} {error [list ownership-collision fallback ::emit]}
set nh_guard_scope [namespace current]
set nh_guard_procs {}
set nh_guard_absent {}
foreach nh_guard_name {::nh} {
    if {[namespace exists $nh_guard_name] || [llength [info commands $nh_guard_name]]} {error [list ownership-collision namespace-or-command $nh_guard_name]}
    lappend nh_guard_absent [list namespace-and-command $nh_guard_name]
}
foreach nh_guard_tail {} {
    set nh_guard_name [string trimright $nh_guard_scope :]::$nh_guard_tail
    if {[llength [info commands $nh_guard_name]] || [string length [namespace which -command $nh_guard_tail]]} {error [list ownership-collision procedure $nh_guard_name resolved [namespace which -command $nh_guard_tail]]}
    lappend nh_guard_procs $nh_guard_name
    lappend nh_guard_absent [list procedure $nh_guard_name fallback $nh_guard_tail]
}
foreach nh_guard_name {} {
    if {[namespace exists $nh_guard_name] || [llength [info commands $nh_guard_name]]} {error [list ownership-collision command-or-namespace $nh_guard_name]}
    lappend nh_guard_procs $nh_guard_name
    lappend nh_guard_absent [list command-and-namespace $nh_guard_name]
}
foreach nh_guard_name {nh_owned_nh20261012collision1_2_description} {
    if {[info exists $nh_guard_name] || [llength [info vars $nh_guard_name]] || [llength [trace vinfo $nh_guard_name]]} {error [list ownership-collision frame-variable $nh_guard_name]}
    lappend nh_guard_absent [list frame-variable $nh_guard_name]
}
list ownership-guard execution-namespace $nh_guard_scope absent $nh_guard_absent
```

### nh_operation

```tcl-visible
namespace eval ::nh {proc emit {fixed value} {list $fixed $value}}
interp alias {} ::nh::local {} ::nh::emit {a b}
rename ::nh::local ::nh::renamed
set nh_owned_nh20261012collision1_2_description [interp alias {} ::nh::local]
list $nh_owned_nh20261012collision1_2_description [::nh::renamed {c d}]

```

### nh_cleanup

```tcl-visible
if {$nh_setup_code == 0} {
    foreach nh_guard_name $nh_guard_procs {
        if {[llength [info commands $nh_guard_name]]} {rename $nh_guard_name {}}
    }
    foreach nh_guard_name {::nh} {
        if {[namespace exists $nh_guard_name]} {namespace delete $nh_guard_name}
    }
    foreach nh_guard_name {nh_owned_nh20261012collision1_2_description} {unset -nocomplain $nh_guard_name}
    foreach nh_guard_fallback {::emit} {if {[llength [info commands $nh_guard_fallback]]} {rename $nh_guard_fallback {}}}
    set nh_guard_remaining {}
    foreach nh_guard_name $nh_guard_procs {
        if {[llength [info commands $nh_guard_name]]} {lappend nh_guard_remaining [list command $nh_guard_name]}
    }
    foreach nh_guard_name {::nh} {
        if {[namespace exists $nh_guard_name]} {lappend nh_guard_remaining [list namespace $nh_guard_name]}
    }
    foreach nh_guard_name {nh_owned_nh20261012collision1_2_description} {
        if {[info exists $nh_guard_name]} {lappend nh_guard_remaining [list frame-variable $nh_guard_name]}
    }
    list ownership-cleanup execution-namespace $nh_guard_scope remaining $nh_guard_remaining
} else {
    list ownership-not-acquired original-source-unreached
}
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

- alias-prefix-lifetime, setup, completion 0, result:

```text-visible
ownership-guard execution-namespace :: absent {{namespace-and-command ::nh} {frame-variable nh_owned_nh20261012collision1_2_description}}⟦EOF⟧
```

- alias-prefix-lifetime, lookup, completion 0, result:

```text-visible
{::nh::emit {a b}} {{a b} {c d}}⟦EOF⟧
```

- alias-prefix-lifetime, runtime-cleanup, completion 0, result:

```text-visible
ownership-cleanup execution-namespace :: remaining {}⟦EOF⟧
```

- alias-prefix-lifetime: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-prefix-lifetime, setup, completion 0, result:

```text-visible
ownership-guard execution-namespace :: absent {{namespace-and-command ::nh} {frame-variable nh_owned_nh20261012collision1_2_description}}⟦EOF⟧
```

- alias-prefix-lifetime, lookup, completion 0, result:

```text-visible
{::nh::emit {a b}} {{a b} {c d}}⟦EOF⟧
```

- alias-prefix-lifetime, runtime-cleanup, completion 0, result:

```text-visible
ownership-cleanup execution-namespace :: remaining {}⟦EOF⟧
```

- alias-prefix-lifetime: intended operation reached.
