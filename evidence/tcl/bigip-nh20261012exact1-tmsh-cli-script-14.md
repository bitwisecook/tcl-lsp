# BIG-IP tmsh Tcl: nh20261012exact1, tmsh-cli-script-14

## Question

This script contains the following cases, which investigate Tcl name handling relevant to the LSP, compiler and VM. Each observation is limited to its own case and this execution context.

- **tcltest-shared-lifecycle:** Do tcltest setup, body and cleanup share the expected namespace variable state, and is the setup variable absent after cleanup?

## Assumptions

- **tcltest-shared-lifecycle:** Entire-program evaluation stage only; capture ownership-guard completion does not by itself establish completion of the original program's setup prefix. Final intended lookup reachability is recorded separately. Intended operation: not recorded.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmsh-cli-script-1](fixtures/bigip-tmsh-cli-script-1.md)

- run: `nh20261012exact1`
- case: `tcltest-shared-lifecycle`

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
foreach nh_guard_name {} {
    if {[info exists $nh_guard_name] || [llength [info vars $nh_guard_name]] || [llength [trace vinfo $nh_guard_name]]} {error [list ownership-collision frame-variable $nh_guard_name]}
    lappend nh_guard_absent [list frame-variable $nh_guard_name]
}
if {[string length [package provide tcltest]] && [llength [info commands ::tcltest::configure]]} {
    if {[::tcltest::configure -match] != "*" || [llength [::tcltest::configure -skip]]} {error {ownership-collision pre-existing tcltest selection}}
}
list ownership-guard execution-namespace $nh_guard_scope absent $nh_guard_absent
```

### nh_operation

```tcl-visible
package require tcltest
namespace eval ::nh {
    namespace import ::tcltest::*
    set observed {}
    test name-corpus-1 {} -setup {
        set payload {a b}
    } -body {
        lappend observed [set payload]
    } -cleanup {
        lappend observed [set payload]
        unset payload
    } -result {{a b}}
}
list $::nh::observed [info exists ::nh::payload]

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
    foreach nh_guard_name {} {unset -nocomplain $nh_guard_name}
    set nh_guard_remaining {}
    foreach nh_guard_name $nh_guard_procs {
        if {[llength [info commands $nh_guard_name]]} {lappend nh_guard_remaining [list command $nh_guard_name]}
    }
    foreach nh_guard_name {::nh} {
        if {[namespace exists $nh_guard_name]} {lappend nh_guard_remaining [list namespace $nh_guard_name]}
    }
    foreach nh_guard_name {} {
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

#### tcltest-shared-lifecycle

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
ownership-guard execution-namespace ::script absent {{namespace-and-command ::nh}}⟦EOF⟧
```

- lookup, completion 1, result (utf-8; unit none, invocation once):

```text-visible
can't read "debug": no such variable⟦EOF⟧
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
can't read "debug": no such variable
    while executing
"if {$debug >= $level} {
⟦TAB⟧puts $string
    }"
    (procedure "DebugPuts" line 3)
    invoked from within
"DebugPuts 3 "entering ConstraintInitializer $constraint $script""
    (procedure "ConstraintInitializer" line 3)
    invoked from within
"ConstraintInitializer singleTestInterp {singleProcess}"
    (procedure "DefineConstraintInitializers" line 2)
    invoked from within
"DefineConstraintInitializers"
    (in namespace eval "::tcltest" script line 3)
    invoked from within
"namespace eval tcltest {
    # Define initializers for all the built-in contraint definitions
    DefineConstraintInitializers

    # Set up the const..."
    (file "/usr/share/compat-tcl8.4/tcltest2.2/tcltest.tcl" line 3272)
    invoked from within
"source /usr/share/compat-tcl8.4/tcltest2.2/tcltest.tcl"
    ("package ifneeded" script)
    invoked from within
"package require tcltest"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
ownership-cleanup execution-namespace ::script remaining {}⟦EOF⟧
```
