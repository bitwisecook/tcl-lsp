# BIG-IP Tcl log: nh20261012exact2, tmm-RULE_INIT-11

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **oo-prefix-deleted-near-miss:** What happens to a command prefix containing an object's self command after that object is destroyed: can it still invoke the method or does command lookup fail?

## Assumptions

- **oo-prefix-deleted-near-miss:** Complete byte-identical original source evaluated in the retained host wrapper's actual namespace/frame. Ownership guard precedes the entire program; cleanup deletes only original exact names proven absent. Whole-program completion does not by itself prove that a particular nested/final lookup was reached.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-rule-init-1](fixtures/bigip-tmm-rule-init-1.md)

- run: `nh20261012exact2`
- case: `oo-prefix-deleted-near-miss`

### nh_setup

```tcl-visible
set nh_guard_scope [namespace current]
set nh_guard_procs {}
set nh_guard_absent {}
foreach nh_guard_name {} {
    if {[namespace exists $nh_guard_name] || [llength [info commands $nh_guard_name]]} {error [list ownership-collision namespace-or-command $nh_guard_name]}
    lappend nh_guard_absent [list namespace-and-command $nh_guard_name]
}
foreach nh_guard_tail {} {
    set nh_guard_name [string trimright $nh_guard_scope :]::$nh_guard_tail
    if {[llength [info commands $nh_guard_name]] || [string length [namespace which -command $nh_guard_tail]]} {error [list ownership-collision procedure $nh_guard_name resolved [namespace which -command $nh_guard_tail]]}
    lappend nh_guard_procs $nh_guard_name
    lappend nh_guard_absent [list procedure $nh_guard_name fallback $nh_guard_tail]
}
foreach nh_guard_name {::nhclass} {
    if {[namespace exists $nh_guard_name] || [llength [info commands $nh_guard_name]]} {error [list ownership-collision command-or-namespace $nh_guard_name]}
    lappend nh_guard_procs $nh_guard_name
    lappend nh_guard_absent [list command-and-namespace $nh_guard_name]
}
foreach nh_guard_name {object prefix result} {
    if {[info exists $nh_guard_name] || [llength [info vars $nh_guard_name]] || [llength [trace vinfo $nh_guard_name]]} {error [list ownership-collision frame-variable $nh_guard_name]}
    lappend nh_guard_absent [list frame-variable $nh_guard_name]
}
list ownership-guard execution-namespace $nh_guard_scope absent $nh_guard_absent
```

### nh_operation

```tcl-visible
oo::class create ::nhclass {
    method emit {value} {list ok $value}
    method prefix {} {list [self] emit {a b}}
}
set object [::nhclass new]
set prefix [$object prefix]
$object destroy
catch {eval $prefix} result
string match {invalid command name *} $result

```

### nh_cleanup

```tcl-visible
if {$nh_setup_code == 0} {
    foreach nh_guard_name $nh_guard_procs {
        if {[llength [info commands $nh_guard_name]]} {rename $nh_guard_name {}}
    }
    foreach nh_guard_name {} {
        if {[namespace exists $nh_guard_name]} {namespace delete $nh_guard_name}
    }
    foreach nh_guard_name {object prefix result} {unset -nocomplain $nh_guard_name}
    set nh_guard_remaining {}
    foreach nh_guard_name $nh_guard_procs {
        if {[llength [info commands $nh_guard_name]]} {lappend nh_guard_remaining [list command $nh_guard_name]}
    }
    foreach nh_guard_name {} {
        if {[namespace exists $nh_guard_name]} {lappend nh_guard_remaining [list namespace $nh_guard_name]}
    }
    foreach nh_guard_name {object prefix result} {
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

- oo-prefix-deleted-near-miss, setup, completion 1, result:

```text-visible
ownership-collision frame-variable prefix⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, errorinfo:

```text-visible
ownership-collision frame-variable prefix
    while executing
"error [list ownership-collision frame-variable $nh_guard_name]"
    ("foreach" body line 2)
    invoked from within
"foreach nh_guard_name {object prefix result} {
    if {[info exists $nh_guard_name] || [llength [info vars $nh_guard_name]] || [llength [trace vinfo $..."
    ("eval" body line 19)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- oo-prefix-deleted-near-miss, runtime-cleanup, completion 0, result:

```text-visible
ownership-not-acquired original-source-unreached⟦EOF⟧
```

- oo-prefix-deleted-near-miss: intended operation not reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- oo-prefix-deleted-near-miss, setup, completion 1, result:

```text-visible
ownership-collision frame-variable prefix⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, errorinfo:

```text-visible
ownership-collision frame-variable prefix
    while executing
"error [list ownership-collision frame-variable $nh_guard_name]"
    ("foreach" body line 2)
    invoked from within
"foreach nh_guard_name {object prefix result} {
    if {[info exists $nh_guard_name] || [llength [info vars $nh_guard_name]] || [llength [trace vinfo $..."
    ("eval" body line 19)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- oo-prefix-deleted-near-miss, runtime-cleanup, completion 0, result:

```text-visible
ownership-not-acquired original-source-unreached⟦EOF⟧
```

- oo-prefix-deleted-near-miss: intended operation not reached.
