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
