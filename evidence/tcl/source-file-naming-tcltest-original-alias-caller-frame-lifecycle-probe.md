# Tcl source-file execution evidence

## Question

> Tcltest setup, body and cleanup source reads must remain associated with the actual selected package and original caller through import and captured alias routes. Treating each script as an unrelated source body can report the original caller variables as undefined. This public experiment asks which values the original loaded package makes visible; it does not execute the reported tclopt constructors or validate Compiler diagnostics.

> For the exact loaded original Tcltest package and fixed original CLI source, do qualified, imported, direct interp alias and captured-name/description alias invocations preserve setup/body/cleanup caller-visible values? What actual namespace-imported caller result is observed? What independent global-test/qualified-wrapper availability and setup-error behavior is observed on the other package implementation?

This is one source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

> The ASCII LF CLI script uses real package loading, interpreter aliases and public catch/list results. It substitutes benign variable names and does not run the original tclopt constructors. Compiler version and unlisted inherited environment were not recorded. Private frame or object identity, general Tcltest behavior, BIG-IP behavior and Rust assertions are outside this observation.

Other assumptions were not separately recorded.

## Script

```tcl-visible
# Fixed ASCII public Tcltest alias/caller-frame observations.
# Source-only W210 fixtures and Native execution permissions remain separate.
set loaded_code [catch {package require tcltest} loaded_version]
puts [list PACKAGE $loaded_code $loaded_version]
if {$loaded_code != 0} {
    puts {LIFECYCLE_UNAVAILABLE package-load}
    return
}
set c_wrapper [llength [info commands ::tcltest::test]]
set global_wrapper [llength [info commands test]]
puts [list WRAPPERS $c_wrapper $global_wrapper]
if {$c_wrapper} {
    ::tcltest::configure -verbose {}
    namespace import -force ::tcltest::test
    interp alias {} original_direct {} ::tcltest::test
    interp alias {} original_captured {} ::tcltest::test captured {shared caller}
    proc original_caller {mode} {
        set events {}
        set setup {
            lappend events setup
            set pdata PDATA
            set par0 PAR0
            set par1 PAR1
            set pars [list $par0 $par1]
            set optimizer OPTIMIZER
        }
        set body {
            lappend events body $optimizer $pdata $par0 $par1 $pars
            set absent_seen [info exists absent]
            set literal_result {[set data_only DATA]}
        }
        set cleanup {
            lappend events cleanup $optimizer $pdata $par0 $par1 $pars
            set cleanup_seen $pdata
        }
        set expected {[set data_only DATA]}
        switch -- $mode {
            qualified {
                ::tcltest::test qualified {shared caller} -setup $setup -body $body -cleanup $cleanup -result $expected
            }
            imported {
                test imported {shared caller} -setup $setup -body $body -cleanup $cleanup -result $expected
            }
            direct_alias {
                original_direct direct {shared caller} -setup $setup -body $body -cleanup $cleanup -result $expected
            }
            captured_alias {
                original_captured -setup $setup -body $body -cleanup $cleanup -result $expected
            }
        }
        return [list $events $cleanup_seen $absent_seen [info exists data_only]]
    }
    foreach mode {qualified imported direct_alias captured_alias} {
        set code [catch {original_caller $mode} value]
        puts [list C_CALLER $mode $code $value]
    }
    namespace eval original_imported {
        namespace import -force ::tcltest::test
        proc caller {} {
            set events {}
            test namespaced {shared caller} -setup {set prepared READY; lappend events setup} -body {lappend events body $prepared; set prepared} -cleanup {lappend events cleanup $prepared} -result READY
            return [list $events $prepared]
        }
    }
    set code [catch {original_imported::caller} value]
    puts [list C_NAMESPACE_CALLER $code $value]
} elseif {$global_wrapper} {
    # Independently advertised global test protocol, including Jim1.0's
    # setup-error behavior. No C-qualified provider identity is borrowed.
    set ::testinfo(verbose) 0
    proc original_global_caller {} {
        set events {}
        test original_global {} -setup {lappend events setup; set prepared READY} -body {lappend events body $prepared; set prepared} -cleanup {lappend events cleanup $prepared} -result READY
        return [list $events $prepared]
    }
    proc original_global_setup_error {} {
        set events {}
        test original_setup_error {} -setup {lappend events setup; error setup-error} -body {lappend events body; return ok} -cleanup {lappend events cleanup} -result ok
        return $events
    }
    set code [catch {original_global_caller} value]
    puts [list GLOBAL_CALLER $code $value]
    set code [catch {original_global_setup_error} value]
    puts [list GLOBAL_SETUP_ERROR $code $value]
    puts {C_ALIAS_LIFECYCLE_UNAVAILABLE no-qualified-wrapper}
} else {
    puts {LIFECYCLE_UNAVAILABLE no-original-wrapper}
}
```

## Results

### tcl8.4, run 1

- Recorded version: 8.4.20
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
version 8.4.20 initial_class {} alias_command {}
oo_package 1 {can't find package oo} class_command {}
jim_class_controls not_applicable no_class_command
```

- Standard error: empty.

### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
version 8.5.19 initial_class {} alias_command {}
oo_package 1 {can't find package oo} class_command {}
jim_class_controls not_applicable no_class_command
```

- Standard error: empty.

### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
version 8.6.18 initial_class {} alias_command {}
oo_package 1 {can't find package oo} class_command {}
jim_class_controls not_applicable no_class_command
```

- Standard error: empty.

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
version 9.0.4 initial_class {} alias_command {}
oo_package 1 {can't find package oo} class_command {}
jim_class_controls not_applicable no_class_command
```

- Standard error: empty.

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
version 9.1.0 initial_class {} alias_command {}
oo_package 1 {can't find package oo} class_command {}
jim_class_controls not_applicable no_class_command
```

- Standard error: empty.

### jim, run 6

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
version 0.84-9-g5bac7c9 initial_class class alias_command alias
oo_package 0 1.0 class_command class
sequential_members 0 {FIRST SECOND 1}
two_word_before_after 0 {BEFORE AFTER}
moved_earlier_two_word 1 {MovedProc, unknown method "before": should be classname, classvars, constructor, defaultconstructor, destroy, eval, finalize, get, method, methods, new, vars}
bases_current 0 {BASE 11 22}
bases_moved_factory 1 {invalid command name "HeldBase inherited"}
bases_command_alias 1 {invalid command name "AliasName inherited"}
bases_known_deleted 1 {invalid command name "DeletedBase"}
bases_known_nonclass 1 {missing value to go with key}
```

- Standard error: empty.
