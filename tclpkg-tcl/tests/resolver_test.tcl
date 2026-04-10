#!/usr/bin/env tclsh
# Tests for tclpkg MVS resolver.

package require Tcl 8.6
lappend auto_path [file join [file dirname [info script]] .. lib]
package require tclpkg

package require tcltest
namespace import ::tcltest::*

test resolver-single "Single direct dep" -body {
    set result [::tclpkg::resolver::resolve \
        [list [dict create name json minimum 1.0.0]]]
    llength $result
} -result 1

test resolver-two-deps "Two independent deps" -body {
    set result [::tclpkg::resolver::resolve \
        [list \
            [dict create name json minimum 1.0.0] \
            [dict create name http minimum 2.0.0]]]
    set names {}
    foreach r $result { lappend names [dict get $r name] }
    lsort $names
} -result {http json}

test resolver-mvs-max "MVS picks max of minimums" -body {
    # a requires shared@1.0, b requires shared@2.0 → shared@2.0
    proc _test_provider {name version} {
        switch -- $name {
            a { return [list [dict create name shared minimum 1.0.0]] }
            b { return [list [dict create name shared minimum 2.0.0]] }
            default { return {} }
        }
    }
    set result [::tclpkg::resolver::resolve \
        [list \
            [dict create name a minimum 1.0.0] \
            [dict create name b minimum 1.0.0]] \
        {} \
        -provider _test_provider]
    set shared_ver ""
    foreach r $result {
        if {[dict get $r name] eq "shared"} {
            set shared_ver [dict get $r version]
        }
    }
    rename _test_provider ""
    set shared_ver
} -result "2.0.0"

test resolver-replace "Replace overrides version" -body {
    set result [::tclpkg::resolver::resolve \
        [list [dict create name json minimum 1.0.0]] \
        {} \
        -replaces [list [dict create name json version 2.0.0]]]
    dict get [lindex $result 0] version
} -result "2.0.0"

test resolver-exclude "Exclude raises on selected version" -body {
    catch {
        ::tclpkg::resolver::resolve \
            [list [dict create name json minimum 1.0.0]] \
            {} \
            -excludes [list [dict create name json version 1.0.0]]
    } err
    string match "*excluded*" $err
} -result 1

test resolver-dev-included "Dev deps included by default" -body {
    set result [::tclpkg::resolver::resolve \
        [list [dict create name json minimum 1.0.0]] \
        [list [dict create name tcltest minimum 2.5.0]]]
    set names {}
    foreach r $result { lappend names [dict get $r name] }
    expr {"tcltest" in $names}
} -result 1

test resolver-dev-excluded "Dev deps excluded with flag" -body {
    set result [::tclpkg::resolver::resolve \
        [list [dict create name json minimum 1.0.0]] \
        [list [dict create name tcltest minimum 2.5.0]] \
        -include_dev 0]
    set names {}
    foreach r $result { lappend names [dict get $r name] }
    expr {"tcltest" in $names}
} -result 0

test resolver-sorted "Result sorted by name" -body {
    set result [::tclpkg::resolver::resolve \
        [list \
            [dict create name zlib minimum 1.0.0] \
            [dict create name alpha minimum 1.0.0] \
            [dict create name mid minimum 1.0.0]]]
    set names {}
    foreach r $result { lappend names [dict get $r name] }
    expr {$names eq [lsort $names]}
} -result 1

cleanupTests
