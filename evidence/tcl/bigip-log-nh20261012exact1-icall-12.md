# BIG-IP Tcl log: nh20261012exact1, icall-12

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **oo-self-prefix:** Can a TclOO method return a list prefix containing its own object command and a multiword argument for later method invocation?

## Assumptions

- **oo-self-prefix:** Entire-program evaluation stage only; capture ownership-guard completion does not by itself establish completion of the original program's setup prefix. Final intended lookup reachability is recorded separately.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-icall-1](fixtures/bigip-icall-1.md)

- run: `nh20261012exact1`
- case: `oo-self-prefix`

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
foreach nh_guard_name {object} {
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
eval [$object prefix]

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
    foreach nh_guard_name {object} {unset -nocomplain $nh_guard_name}
    set nh_guard_remaining {}
    foreach nh_guard_name $nh_guard_procs {
        if {[llength [info commands $nh_guard_name]]} {lappend nh_guard_remaining [list command $nh_guard_name]}
    }
    foreach nh_guard_name {} {
        if {[namespace exists $nh_guard_name]} {lappend nh_guard_remaining [list namespace $nh_guard_name]}
    }
    foreach nh_guard_name {object} {
        if {[info exists $nh_guard_name]} {lappend nh_guard_remaining [list frame-variable $nh_guard_name]}
    }
    list ownership-cleanup execution-namespace $nh_guard_scope remaining $nh_guard_remaining
} else {
    list ownership-not-acquired original-source-unreached
}
```

## Results

### icall, unit none, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: icall.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- oo-self-prefix, setup, completion 0, result:

```text-visible
ownership-guard execution-namespace :: absent {{command-and-namespace ::nhclass} {frame-variable object}}⟦EOF⟧
```

- oo-self-prefix, lookup, completion 1, result:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- oo-self-prefix, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- oo-self-prefix, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- oo-self-prefix, lookup, completion 1, errorinfo:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::nhclass {
    method emit {value} {list ok $value}
    method prefix {} {list [self] emit {a b}}
}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- oo-self-prefix, runtime-cleanup, completion 0, result:

```text-visible
ownership-cleanup execution-namespace :: remaining {}⟦EOF⟧
```

- oo-self-prefix: intended operation not reached.
