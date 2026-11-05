# BIG-IP Tcl log: nh20261012collision1, tmm-RULE_INIT-6

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **string-prefix-near-miss:** What happens when a command name containing spaces is interpolated into an eval script string rather than preserved as a list command-prefix element?

## Assumptions

- **string-prefix-near-miss:** Complete separately adapted program in actual host. Intended original final lookup is not inferred from whole-program completion. Enumerated spelling substitutions avoid unowned native commands/frame variables.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-rule-init-1](fixtures/bigip-tmm-rule-init-1.md)

- run: `nh20261012collision1`
- case: `string-prefix-near-miss`

### nh_setup

```tcl-visible
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
foreach nh_guard_name {nh_owned_nh20261012collision1_6_target} {
    if {[info exists $nh_guard_name] || [llength [info vars $nh_guard_name]] || [llength [trace vinfo $nh_guard_name]]} {error [list ownership-collision frame-variable $nh_guard_name]}
    lappend nh_guard_absent [list frame-variable $nh_guard_name]
}
list ownership-guard execution-namespace $nh_guard_scope absent $nh_guard_absent
```

### nh_operation

```tcl-visible
namespace eval ::nh {}
proc {::nh::two words} {fixed value} {list $fixed $value}
set nh_owned_nh20261012collision1_6_target {::nh::two words}
eval "$nh_owned_nh20261012collision1_6_target {a b} {c d}"

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
    foreach nh_guard_name {nh_owned_nh20261012collision1_6_target} {unset -nocomplain $nh_guard_name}
    foreach nh_guard_fallback {} {if {[llength [info commands $nh_guard_fallback]]} {rename $nh_guard_fallback {}}}
    set nh_guard_remaining {}
    foreach nh_guard_name $nh_guard_procs {
        if {[llength [info commands $nh_guard_name]]} {lappend nh_guard_remaining [list command $nh_guard_name]}
    }
    foreach nh_guard_name {::nh} {
        if {[namespace exists $nh_guard_name]} {lappend nh_guard_remaining [list namespace $nh_guard_name]}
    }
    foreach nh_guard_name {nh_owned_nh20261012collision1_6_target} {
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

- string-prefix-near-miss, setup, completion 0, result:

```text-visible
ownership-guard execution-namespace :: absent {{namespace-and-command ::nh} {frame-variable nh_owned_nh20261012collision1_6_target}}⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, result:

```text-visible
invalid command name "::nh::two"⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorinfo:

```text-visible
invalid command name "::nh::two"
    while executing
"::nh::two words {a b} {c d}"
    ("eval" body line 1)
    invoked from within
"eval "$nh_owned_nh20261012collision1_6_target {a b} {c d}""
    ("eval" body line 4)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- string-prefix-near-miss, runtime-cleanup, completion 0, result:

```text-visible
ownership-cleanup execution-namespace :: remaining {}⟦EOF⟧
```

- string-prefix-near-miss: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- string-prefix-near-miss, setup, completion 0, result:

```text-visible
ownership-guard execution-namespace :: absent {{namespace-and-command ::nh} {frame-variable nh_owned_nh20261012collision1_6_target}}⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, result:

```text-visible
invalid command name "::nh::two"⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorinfo:

```text-visible
invalid command name "::nh::two"
    while executing
"::nh::two words {a b} {c d}"
    ("eval" body line 1)
    invoked from within
"eval "$nh_owned_nh20261012collision1_6_target {a b} {c d}""
    ("eval" body line 4)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- string-prefix-near-miss, runtime-cleanup, completion 0, result:

```text-visible
ownership-cleanup execution-namespace :: remaining {}⟦EOF⟧
```

- string-prefix-near-miss: intended operation reached.
