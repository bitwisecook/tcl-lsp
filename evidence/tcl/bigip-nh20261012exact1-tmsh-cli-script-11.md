# BIG-IP tmsh Tcl: nh20261012exact1, tmsh-cli-script-11

## Question

This script contains the following cases, which investigate Tcl name handling relevant to the LSP, compiler and VM. Each observation is limited to its own case and this execution context.

- **oo-prefix-deleted-near-miss:** What happens to a command prefix containing an object's self command after that object is destroyed: can it still invoke the method or does command lookup fail?

## Assumptions

- **oo-prefix-deleted-near-miss:** Entire-program evaluation stage only; capture ownership-guard completion does not by itself establish completion of the original program's setup prefix. Final intended lookup reachability is recorded separately. Intended operation: not reached.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmsh-cli-script-1](fixtures/bigip-tmsh-cli-script-1.md)

- run: `nh20261012exact1`
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

### BIG-IP tmsh CLI script

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmsh run cli script.
- Exit status: 0
- Standard output: decoded below.

- Standard error: empty.

The exit status and streams are from the recorded tmsh invocation. The entries below decode complete NHO payloads from its standard output.

#### oo-prefix-deleted-near-miss

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
ownership-guard execution-namespace ::script absent {{command-and-namespace ::nhclass} {frame-variable object} {frame-variable prefix} {frame-variable result}}⟦EOF⟧
```

- lookup, completion 1, result (utf-8; unit none, invocation once):

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- lookup, completion 1, metadata (utf-8; unit none, invocation once):

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- lookup, completion 1, errorcode (utf-8; unit none, invocation once):

```text-visible
NONE⟦EOF⟧
```

- lookup, completion 1, errorinfo (utf-8; unit none, invocation once):

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

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
ownership-cleanup execution-namespace ::script remaining {}⟦EOF⟧
```
