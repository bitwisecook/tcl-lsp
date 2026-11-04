# Tcl Wiki naming patterns across Tcl versions

## Question

This script examines the following Tcl naming and execution distinctions that affect LSP navigation and diagnostics, compiler transformations, and VM command and variable lookup. Each case answers only its stated question.

- **wiki-prefix-1:** Does invoking a list command prefix preserve arguments containing spaces and other embedded data as individual argument values?

- **wiki-prefix-2:** How does a newline in a command prefix behave when the prefix is evaluated as a script versus expanded as a list of command words?

- **wiki-prefix-3:** Can a command spelling that looks like a list-prefix builder be shadowed, and what does that mean for proving the constructed command words statically?

- **wiki-code-1:** Does namespace code capture the command lookup namespace when its result is later invoked from another namespace?

- **wiki-code-2:** Does namespace code capture a procedure's local variables, or only its namespace for later lookup?

- **wiki-code-3:** When a command used by namespace code is replaced after the callback is created, does invoking the callback use the original or the replacement command?

- **wiki-code-4:** What happens when a namespace inscope callback is invoked after its target namespace has been deleted?

- **wiki-code-5:** After a captured namespace is deleted and a namespace with the same name is recreated, which namespace does the old callback address?

- **wiki-code-6:** Can the legacy unknown-command handler repair an invocation of nested namespace code, and under which interpreter constraint?

- **wiki-code-7:** Does the modern unknown-command handler accept or reject nested namespace code, and under which interpreter constraint?

- **wiki-factory-1:** Do procedures created by a finite factory with list-constructed bodies retain the factory's individual constant values?

- **wiki-factory-2:** Does a braced procedure body created by a factory capture the factory's loop-local variable for later invocation?

- **wiki-alias-1:** When an alias target command is replaced, does the alias follow the target name while an imported command retains the earlier command identity?

- **wiki-alias-2:** Which interpreter supplies command lookup for an alias target when the alias crosses an interpreter boundary?

- **wiki-rename-1:** Does renaming a procedure into another namespace change the namespace context used when its body executes?

- **wiki-upvar-1:** How does upvar resolve a digit-leading variable name when the caller specifies the frame level explicitly?

- **wiki-upvar-2:** How does upvar distinguish an implicit frame level from a digit-leading variable name across Tcl releases?

- **wiki-upvar-3:** Can an upvar wrapper forward the caller frame so that it links the intended caller variable?

- **wiki-upvar-4:** Which frame receives an upvar link when a wrapper invokes upvar without forwarding the caller frame?

- **wiki-variable-1:** How does variable interpret alternating name and value arguments in a namespace declaration?

- **wiki-global-1:** Must a wrapper execute global in the caller's frame for the caller to receive the global-variable link?

- **wiki-autoload-1:** Can lookup of a missing command load its implementation from auto_index and then invoke it?

- **wiki-unknown-1:** Can an unknown handler make a previously absent command head executable, and how does that affect lookup?

- **wiki-oo-1:** Does namespace code preserve private my method dispatch when a callback created in an object context is invoked later?

- **wiki-oo-2:** What happens to a captured my callback when its object has been deleted?

- **wiki-oo-3:** Does a TclOO forward follow a command name when the target command is replaced?

- **wiki-oo-4:** Can metaclass methods dynamically define methods on a class, and how are those methods dispatched?

- **wiki-oo-5:** Can a metaclass override new to return a value that is not an object?

- **wiki-oo-6:** Does a TclOO forward through my resolve an internal method in the object context?

- **wiki-oo-7:** Does replacing a TclOO method preserve or reset that method's explicit export state?

## Assumptions

Each test creates a fresh child interpreter. The real tcltest package supplies the test runner. A skipped constraint is not an observation of the skipped behavior. The Wiki examples motivate the reductions; they do not establish runtime behavior themselves. BIG-IP and Jim were not measured in this collection. Other assumptions were not separately recorded.

## Script

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

## Results

### Tcl 8.4.20

- Recorded version: 8.4.20
- Recorded input channel: script file passed to tclsh.
- Exit status: 0
- Standard output:

```text-visible
oracle 8.4.20 library /workspace/tcl-lsp/tmp/tcl8.4.20/library tcltest 2.2.11
++++ wiki-prefix-1 PASSED
++++ wiki-prefix-2 SKIPPED: tcl85
++++ wiki-prefix-3 PASSED
++++ wiki-code-1 PASSED
++++ wiki-code-2 PASSED
++++ wiki-code-3 PASSED
++++ wiki-code-4 PASSED
++++ wiki-code-5 PASSED
++++ wiki-code-6 PASSED
++++ wiki-code-7 SKIPPED: noAunt
++++ wiki-factory-1 PASSED
++++ wiki-factory-2 PASSED
++++ wiki-alias-1 PASSED
++++ wiki-alias-2 PASSED
++++ wiki-rename-1 PASSED
++++ wiki-upvar-1 PASSED
++++ wiki-upvar-2 PASSED
++++ wiki-upvar-3 PASSED
++++ wiki-upvar-4 PASSED
++++ wiki-variable-1 PASSED
++++ wiki-global-1 PASSED
++++ wiki-autoload-1 PASSED
++++ wiki-unknown-1 PASSED
++++ wiki-oo-1 SKIPPED: tcloo
++++ wiki-oo-2 SKIPPED: tcloo
++++ wiki-oo-3 SKIPPED: tcloo
++++ wiki-oo-4 SKIPPED: tcloo
++++ wiki-oo-5 SKIPPED: tcloo
++++ wiki-oo-6 SKIPPED: tcloo
++++ wiki-oo-7 SKIPPED: tcloo
wiki-probes.tcl:⟦TAB⟧Total⟦TAB⟧30⟦TAB⟧Passed⟦TAB⟧21⟦TAB⟧Skipped⟦TAB⟧9⟦TAB⟧Failed⟦TAB⟧0
Number of tests skipped for each constraint:
⟦TAB⟧1⟦TAB⟧noAunt
⟦TAB⟧1⟦TAB⟧tcl85
⟦TAB⟧7⟦TAB⟧tcloo
```

- Standard error: empty.

### Tcl 8.5.19

- Recorded version: 8.5.19
- Recorded input channel: script file passed to tclsh.
- Exit status: 0
- Standard output:

```text-visible
oracle 8.5.19 library /workspace/tcl-lsp/tmp/tcl8.5.19/library tcltest 2.3.8
++++ wiki-prefix-1 PASSED
++++ wiki-prefix-2 PASSED
++++ wiki-prefix-3 PASSED
++++ wiki-code-1 PASSED
++++ wiki-code-2 PASSED
++++ wiki-code-3 PASSED
++++ wiki-code-4 PASSED
++++ wiki-code-5 PASSED
++++ wiki-code-6 PASSED
++++ wiki-code-7 SKIPPED: noAunt
++++ wiki-factory-1 PASSED
++++ wiki-factory-2 PASSED
++++ wiki-alias-1 PASSED
++++ wiki-alias-2 PASSED
++++ wiki-rename-1 PASSED
++++ wiki-upvar-1 PASSED
++++ wiki-upvar-2 PASSED
++++ wiki-upvar-3 PASSED
++++ wiki-upvar-4 PASSED
++++ wiki-variable-1 PASSED
++++ wiki-global-1 PASSED
++++ wiki-autoload-1 PASSED
++++ wiki-unknown-1 PASSED
++++ wiki-oo-1 SKIPPED: tcloo
++++ wiki-oo-2 SKIPPED: tcloo
++++ wiki-oo-3 SKIPPED: tcloo
++++ wiki-oo-4 SKIPPED: tcloo
++++ wiki-oo-5 SKIPPED: tcloo
++++ wiki-oo-6 SKIPPED: tcloo
++++ wiki-oo-7 SKIPPED: tcloo
wiki-probes.tcl:⟦TAB⟧Total⟦TAB⟧30⟦TAB⟧Passed⟦TAB⟧22⟦TAB⟧Skipped⟦TAB⟧8⟦TAB⟧Failed⟦TAB⟧0
Number of tests skipped for each constraint:
⟦TAB⟧1⟦TAB⟧noAunt
⟦TAB⟧7⟦TAB⟧tcloo
```

- Standard error: empty.

### Tcl 8.6.18

- Recorded version: 8.6.18
- Recorded input channel: script file passed to tclsh.
- Exit status: 0
- Standard output:

```text-visible
oracle 8.6.18 library /workspace/tcl-lsp/tmp/tcl8.6.18/library tcltest 2.5.11
++++ wiki-prefix-1 PASSED
++++ wiki-prefix-2 PASSED
++++ wiki-prefix-3 PASSED
++++ wiki-code-1 PASSED
++++ wiki-code-2 PASSED
++++ wiki-code-3 PASSED
++++ wiki-code-4 PASSED
++++ wiki-code-5 PASSED
++++ wiki-code-6 SKIPPED: aunt
++++ wiki-code-7 PASSED
++++ wiki-factory-1 PASSED
++++ wiki-factory-2 PASSED
++++ wiki-alias-1 PASSED
++++ wiki-alias-2 PASSED
++++ wiki-rename-1 PASSED
++++ wiki-upvar-1 PASSED
++++ wiki-upvar-2 PASSED
++++ wiki-upvar-3 PASSED
++++ wiki-upvar-4 PASSED
++++ wiki-variable-1 PASSED
++++ wiki-global-1 PASSED
++++ wiki-autoload-1 PASSED
++++ wiki-unknown-1 PASSED
++++ wiki-oo-1 PASSED
++++ wiki-oo-2 PASSED
++++ wiki-oo-3 PASSED
++++ wiki-oo-4 PASSED
++++ wiki-oo-5 PASSED
++++ wiki-oo-6 PASSED
++++ wiki-oo-7 PASSED
wiki-probes.tcl:⟦TAB⟧Total⟦TAB⟧30⟦TAB⟧Passed⟦TAB⟧29⟦TAB⟧Skipped⟦TAB⟧1⟦TAB⟧Failed⟦TAB⟧0
Number of tests skipped for each constraint:
⟦TAB⟧1⟦TAB⟧aunt
```

- Standard error: empty.

### Tcl 9.0.4

- Recorded version: 9.0.4
- Recorded input channel: script file passed to tclsh.
- Exit status: 0
- Standard output:

```text-visible
oracle 9.0.4 library //zipfs:/app/tcl_library tcltest 2.5.11
++++ wiki-prefix-1 PASSED
++++ wiki-prefix-2 PASSED
++++ wiki-prefix-3 PASSED
++++ wiki-code-1 PASSED
++++ wiki-code-2 PASSED
++++ wiki-code-3 PASSED
++++ wiki-code-4 PASSED
++++ wiki-code-5 PASSED
++++ wiki-code-6 SKIPPED: aunt
++++ wiki-code-7 PASSED
++++ wiki-factory-1 PASSED
++++ wiki-factory-2 PASSED
++++ wiki-alias-1 PASSED
++++ wiki-alias-2 PASSED
++++ wiki-rename-1 PASSED
++++ wiki-upvar-1 PASSED
++++ wiki-upvar-2 PASSED
++++ wiki-upvar-3 PASSED
++++ wiki-upvar-4 PASSED
++++ wiki-variable-1 PASSED
++++ wiki-global-1 PASSED
++++ wiki-autoload-1 PASSED
++++ wiki-unknown-1 PASSED
++++ wiki-oo-1 PASSED
++++ wiki-oo-2 PASSED
++++ wiki-oo-3 PASSED
++++ wiki-oo-4 PASSED
++++ wiki-oo-5 PASSED
++++ wiki-oo-6 PASSED
++++ wiki-oo-7 PASSED
wiki-probes.tcl:⟦TAB⟧Total⟦TAB⟧30⟦TAB⟧Passed⟦TAB⟧29⟦TAB⟧Skipped⟦TAB⟧1⟦TAB⟧Failed⟦TAB⟧0
Number of tests skipped for each constraint:
⟦TAB⟧1⟦TAB⟧aunt
```

- Standard error: empty.

### Tcl 9.1.0

- Recorded version: 9.1.0
- Recorded input channel: script file passed to tclsh.
- Exit status: 0
- Standard output:

```text-visible
oracle 9.1.0 library //zipfs:/app/tcl_library tcltest 2.6.0
++++ wiki-prefix-1 PASSED
++++ wiki-prefix-2 PASSED
++++ wiki-prefix-3 PASSED
++++ wiki-code-1 PASSED
++++ wiki-code-2 PASSED
++++ wiki-code-3 PASSED
++++ wiki-code-4 PASSED
++++ wiki-code-5 PASSED
++++ wiki-code-6 SKIPPED: aunt
++++ wiki-code-7 PASSED
++++ wiki-factory-1 PASSED
++++ wiki-factory-2 PASSED
++++ wiki-alias-1 PASSED
++++ wiki-alias-2 PASSED
++++ wiki-rename-1 PASSED
++++ wiki-upvar-1 PASSED
++++ wiki-upvar-2 PASSED
++++ wiki-upvar-3 PASSED
++++ wiki-upvar-4 PASSED
++++ wiki-variable-1 PASSED
++++ wiki-global-1 PASSED
++++ wiki-autoload-1 PASSED
++++ wiki-unknown-1 PASSED
++++ wiki-oo-1 PASSED
++++ wiki-oo-2 PASSED
++++ wiki-oo-3 PASSED
++++ wiki-oo-4 PASSED
++++ wiki-oo-5 PASSED
++++ wiki-oo-6 PASSED
++++ wiki-oo-7 PASSED
wiki-probes.tcl:⟦TAB⟧Total⟦TAB⟧30⟦TAB⟧Passed⟦TAB⟧29⟦TAB⟧Skipped⟦TAB⟧1⟦TAB⟧Failed⟦TAB⟧0
Number of tests skipped for each constraint:
⟦TAB⟧1⟦TAB⟧aunt
```

- Standard error: empty.
