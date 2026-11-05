# BIG-IP APL callback: wiki-prefix-2

## Question

How does a newline in a command prefix behave when the prefix is evaluated as a script versus expanded as a list of command words?

This callback addresses only the `wiki-prefix-2` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: Complete byte-identical real-tcltest Wiki suite evaluated in the retained host namespace/frame, with one selected test. Original isolated helper/child interpreters and constraints are unchanged. Package load failure or ownership collision leaves suite/body unreached. Pass/skip output is harness evidence; selected body bytes and inner options remain unavailable without separately labelled instrumentation.

The retained record says the intended operation was not reached; the result records the setup outcome, not an answer to that operation.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

Fixture: [bigip-iapp-apl-1](fixtures/bigip-iapp-apl-1.md)

- run: `nh20261012aplexact1`
- case: `wiki-prefix-2`

### nh_setup

```tcl-visible
set nh_guard_scope [namespace current]
set nh_guard_procs {}
set nh_guard_absent {}
foreach nh_guard_name {} {
    if {[namespace exists $nh_guard_name] || [llength [info commands $nh_guard_name]]} {error [list ownership-collision namespace-or-command $nh_guard_name]}
    lappend nh_guard_absent [list namespace-and-command $nh_guard_name]
}
foreach nh_guard_tail {isolated} {
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
package require tcltest
set nh_guard_tcltest_match [::tcltest::configure -match]
set nh_guard_tcltest_skip [::tcltest::configure -skip]
set nh_guard_tcltest_verbose [::tcltest::configure -verbose]
::tcltest::configure -match wiki-prefix-2 -skip {}
list ownership-guard execution-namespace $nh_guard_scope absent $nh_guard_absent selected wiki-prefix-2 tcltest [package present tcltest]
```

### nh_operation

```tcl-visible
# SPDX-License-Identifier: AGPL-3.0-or-later

package require tcltest
::tcltest::configure -verbose {pass body error skip}
::tcltest::testConstraint tcl85 [expr {[package vcompare [info patchlevel] 8.5] >= 0}]
::tcltest::testConstraint tcloo [expr {[llength [info commands ::oo::class]] != 0}]
::tcltest::testConstraint aunt [expr {[package vcompare [info patchlevel] 8.6] < 0}]
::tcltest::testConstraint noAunt [expr {[package vcompare [info patchlevel] 8.6] >= 0}]

proc isolated {script} {
    set child [interp create]
    set code [catch {interp eval $child $script} result]
    interp delete $child
    if {$code != 0} {return -code $code $result}
    return $result
}

puts [list oracle [info patchlevel] library [info library] tcltest [package present tcltest]]

::tcltest::test wiki-prefix-1 {list prefix preserves embedded data} -body {
    isolated {
        proc take {fixed value} {list $fixed $value}
        set prefix [list take {x; error injected}]
        eval [linsert $prefix end {a b}]
    }
} -result {{x; error injected} {a b}}

::tcltest::test wiki-prefix-2 {script and list prefix treat newline differently} -constraints tcl85 -body {
    isolated {
        proc take args {return $args}
        proc extra {} {return script}
        set prefix "take\nextra"
        list [{*}$prefix] [eval $prefix]
    }
} -result {extra script}

::tcltest::test wiki-prefix-3 {builder spelling alone cannot prove list construction} -body {
    isolated {
        namespace eval n {
            proc list args {return {other}}
            proc take args {return intended}
            proc other args {return actual}
            eval [list take {a b}]
        }
    }
} -result actual

::tcltest::test wiki-code-1 {namespace code captures command lookup namespace} -body {
    isolated {
        proc take args {return global}
        namespace eval n {
            proc take args {return [linsert $args 0 local]}
            set ::prefix [namespace code [list take fixed]]
        }
        eval [linsert $prefix end {a b}]
    }
} -result {local fixed {a b}}

::tcltest::test wiki-code-2 {captured namespace does not capture a proc local} -body {
    isolated {
        namespace eval n {
            variable x namespace
            proc build {} {set x local; namespace code {set x}}
        }
        eval [n::build]
    }
} -result namespace

::tcltest::test wiki-code-3 {captured command spelling observes replacement} -body {
    isolated {
        namespace eval n {proc take {} {return old}; set ::prefix [namespace code take]}
        rename n::take n::saved
        proc n::take {} {return replacement}
        eval $prefix
    }
} -result replacement

::tcltest::test wiki-code-4 {inscope requires a live namespace} -body {
    isolated {
        namespace eval n {set ::prefix [namespace code {set x}]}
        namespace delete n
        catch {eval $prefix}
    }
} -result 1

::tcltest::test wiki-code-5 {namespace name may denote a recreated namespace} -body {
    isolated {
        namespace eval n {set x old; set ::prefix [namespace code {set x}]}
        namespace delete n
        namespace eval n {set x recreated}
        eval $prefix
    }
} -result recreated

::tcltest::test wiki-code-6 {legacy unknown repairs nested namespace code} -constraints aunt -body {
    isolated {
        namespace eval n {proc take x {return $x}; set ::prefix [list [namespace code take] fixed]}
        eval $prefix
    }
} -result fixed

::tcltest::test wiki-code-7 {modern unknown rejects nested namespace code} -constraints noAunt -body {
    isolated {
        namespace eval n {proc take x {return $x}; set ::prefix [list [namespace code take] fixed]}
        catch {eval $prefix}
    }
} -result 1

::tcltest::test wiki-factory-1 {finite factory substitutes constant body data} -body {
    isolated {
        namespace eval n {
            foreach name {alpha beta} {proc $name {} [list return $name]}
        }
        list [n::alpha] [n::beta]
    }
} -result {alpha beta}

::tcltest::test wiki-factory-2 {braced factory body does not capture loop local} -body {
    isolated {
        proc build {} {
            foreach name {alpha beta} {proc ::$name {} {return $name}}
        }
        build
        catch {alpha}
    }
} -result 1

::tcltest::test wiki-alias-1 {alias target name rebinds while import retains command} -body {
    isolated {
        namespace eval n {namespace export take; proc take x {list old $x}}
        namespace eval consumer {namespace import ::n::take}
        interp alias {} alias {} n::take fixed
        rename n::take n::saved
        proc n::take x {list replacement $x}
        list [alias] [consumer::take fixed]
    }
} -result {{replacement fixed} {old fixed}}

::tcltest::test wiki-alias-2 {alias target interpreter owns lookup} -body {
    isolated {
        proc take {} {return main}
        interp create helper
        interp eval helper {proc take {} {return helper}}
        interp alias {} alias helper take
        alias
    }
} -result helper

::tcltest::test wiki-rename-1 {renaming a proc changes its namespace context} -body {
    isolated {
        namespace eval one {variable x one; proc take {} {variable x; return $x}}
        namespace eval two {variable x two}
        rename one::take two::take
        two::take
    }
} -result two

::tcltest::test wiki-upvar-1 {explicit level supports digit-leading names} -body {
    isolated {
        proc assign {name} {upvar 1 $name value; set value changed}
        proc caller {} {set 0a old; assign 0a; set 0a}
        caller
    }
} -result changed

::tcltest::test wiki-upvar-2 {implicit level parsing changes across releases} -body {
    isolated {
        proc assign {name} {upvar $name value; set value changed}
        proc caller {} {set 0a old; catch {assign 0a}}
        caller
    }
} -result [expr {[package vcompare [info patchlevel] 8.6] < 0}]

::tcltest::test wiki-upvar-3 {frame-forwarding wrapper links the caller variable} -body {
    isolated {
        proc link {name local} {uplevel 1 [list upvar 1 $name $local]}
        proc assign {name} {link $name value; set value changed}
        proc caller {} {set x old; assign x; return $x}
        caller
    }
} -result changed

::tcltest::test wiki-upvar-4 {naive wrapper creates a link in its own frame} -body {
    isolated {
        proc link {name local} {upvar 1 $name $local}
        proc assign {name} {link $name value; set value changed}
        proc caller {} {set x old; assign x; return $x}
        caller
    }
} -result old

::tcltest::test wiki-variable-1 {variable alternates names and assigned values} -body {
    isolated {
        namespace eval n {variable a b; list $a [info exists b]}
    }
} -result {b 0}

::tcltest::test wiki-global-1 {global wrapper must execute declaration in caller} -body {
    isolated {
        set x global
        proc link {name} {uplevel 1 [list global $name]}
        proc caller {} {link x; return $x}
        caller
    }
} -result global

::tcltest::test wiki-autoload-1 {a missing command may load from auto_index} -body {
    isolated {
        set auto_index(::late) {proc ::late {} {return loaded}}
        list [info commands ::late] [::late] [info commands ::late]
    }
} -result {{} loaded ::late}

::tcltest::test wiki-unknown-1 {unknown can make an absent head executable} -body {
    isolated {
        proc unknown {head args} {return [linsert $args 0 $head]}
        list [info commands absent] [absent fixed]
    }
} -result {{} {absent fixed}}

::tcltest::test wiki-oo-1 {namespace code preserves private my method dispatch} -constraints tcloo -body {
    isolated {
        oo::class create C {
            method hidden {x} {return $x}
            unexport hidden
            method prefix {} {namespace code [list my hidden fixed]}
        }
        set obj [C new]
        set prefix [$obj prefix]
        list [catch {$obj hidden fixed}] [eval $prefix]
    }
} -result {1 fixed}

::tcltest::test wiki-oo-2 {object deletion invalidates captured my callback} -constraints tcloo -body {
    isolated {
        oo::class create C {method hidden {} {return ok}; method prefix {} {namespace code {my hidden}}}
        set obj [C new]
        set prefix [$obj prefix]
        $obj destroy
        catch {eval $prefix}
    }
} -result 1

::tcltest::test wiki-oo-3 {forward target observes command replacement} -constraints tcloo -body {
    isolated {
        proc take x {list old $x}
        oo::class create C {forward send ::take fixed}
        set obj [C new]
        rename take saved
        proc take x {list replacement $x}
        $obj send
    }
} -result {replacement fixed}

::tcltest::test wiki-oo-4 {metaclass methods can define class methods dynamically} -constraints tcloo -body {
    isolated {
        oo::class create Meta {
            superclass oo::class
            method method {name params body} {::oo::define [self] method $name $params $body}
        }
        Meta create C
        C method value {} {return generated}
        set obj [C new]
        $obj value
    }
} -result generated

::tcltest::test wiki-oo-5 {metaclass can override new to return a non-object} -constraints tcloo -body {
    isolated {
        oo::class create Meta {superclass oo::class; method new args {return ordinary}}
        Meta create C
        set value [C new]
        list $value [info object isa object $value]
    }
} -result {ordinary 0}

::tcltest::test wiki-oo-6 {forward my resolves an internal method} -constraints tcloo -body {
    isolated {
        oo::class create C {
            method hidden {} {return internal}
            unexport hidden
            forward send my hidden
        }
        set obj [C new]
        list [catch {$obj hidden}] [$obj send]
    }
} -result {1 internal}

::tcltest::test wiki-oo-7 {method replacement resets explicit export} -constraints tcloo -body {
    isolated {
        oo::class create C {method .send {} {return old}; export .send}
        set obj [C new]
        set before [$obj .send]
        oo::define C method .send {} {return replacement}
        list $before [catch {$obj .send}]
    }
} -result {old 1}

::tcltest::cleanupTests
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
    foreach nh_guard_name {} {unset -nocomplain $nh_guard_name}
    ::tcltest::configure -match $nh_guard_tcltest_match -skip $nh_guard_tcltest_skip -verbose $nh_guard_tcltest_verbose
    set nh_guard_remaining {}
    foreach nh_guard_name $nh_guard_procs {
        if {[llength [info commands $nh_guard_name]]} {lappend nh_guard_remaining [list command $nh_guard_name]}
    }
    foreach nh_guard_name {} {
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

### BIG-IP iApp APL

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel 8.4.6.
- Recorded input channel: browser Application Services template choice callback.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are the retained DOM option values projected one row per value, not process stdout. The fields below decode their hexadecimal payloads; completion codes are the callback's caught Tcl codes.

- setup, completion 1, result (utf-8; unit none, invocation once):

```text-visible
can't read "debug": no such variable⟦EOF⟧
```

- setup, completion 1, metadata (utf-8; unit none, invocation once):

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- setup, completion 1, errorcode (utf-8; unit none, invocation once):

```text-visible
NONE⟦EOF⟧
```

- setup, completion 1, errorinfo (utf-8; unit none, invocation once):

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
    ("eval" body line 23)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
ownership-not-acquired original-source-unreached⟦EOF⟧
```
