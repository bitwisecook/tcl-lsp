#!/usr/bin/env tclsh
# Tests for tclpkg CAS integrity hashing.

package require Tcl 8.6
lappend auto_path [file join [file dirname [info script]] .. lib]
package require tclpkg

package require tcltest
namespace import ::tcltest::*

proc _make_tree {dir files} {
    file mkdir $dir
    foreach {rel content} $files {
        set path [file join $dir $rel]
        file mkdir [file dirname $path]
        set fd [open $path w]
        puts -nonewline $fd $content
        close $fd
    }
}

test cas-hash-stable "Same tree hashes identically" -setup {
    set d [file join [::tcltest::temporaryDirectory] cas_stable]
    _make_tree $d {a.tcl "set x 1\n" b.tcl "set y 2\n"}
} -body {
    set h1 [::tclpkg::cas::integrity_of_tree $d]
    set h2 [::tclpkg::cas::integrity_of_tree $d]
    expr {$h1 eq $h2}
} -cleanup { file delete -force -- $d } -result 1

test cas-hash-sha256-prefix "Hash starts with sha256-" -setup {
    set d [file join [::tcltest::temporaryDirectory] cas_prefix]
    _make_tree $d {a.tcl "ok\n"}
} -body {
    string match "sha256-*" [::tclpkg::cas::integrity_of_tree $d]
} -cleanup { file delete -force -- $d } -result 1

test cas-hash-different-content "Different content produces different hash" -setup {
    set d1 [file join [::tcltest::temporaryDirectory] cas_diff1]
    set d2 [file join [::tcltest::temporaryDirectory] cas_diff2]
    _make_tree $d1 {a.tcl "set x 1\n"}
    _make_tree $d2 {a.tcl "set x 2\n"}
} -body {
    set h1 [::tclpkg::cas::integrity_of_tree $d1]
    set h2 [::tclpkg::cas::integrity_of_tree $d2]
    expr {$h1 ne $h2}
} -cleanup { file delete -force -- $d1; file delete -force -- $d2 } -result 1

test cas-hash-same-content "Same content in different dirs produces same hash" -setup {
    set d1 [file join [::tcltest::temporaryDirectory] cas_same1]
    set d2 [file join [::tcltest::temporaryDirectory] cas_same2]
    _make_tree $d1 {a.tcl "hello\n"}
    _make_tree $d2 {a.tcl "hello\n"}
} -body {
    set h1 [::tclpkg::cas::integrity_of_tree $d1]
    set h2 [::tclpkg::cas::integrity_of_tree $d2]
    expr {$h1 eq $h2}
} -cleanup { file delete -force -- $d1; file delete -force -- $d2 } -result 1

test cas-hash-ignores-dotgit "Ignores .git directory" -setup {
    set d1 [file join [::tcltest::temporaryDirectory] cas_git1]
    set d2 [file join [::tcltest::temporaryDirectory] cas_git2]
    _make_tree $d1 {a.tcl "ok\n"}
    _make_tree $d2 {a.tcl "ok\n" .git/HEAD "ref: main\n"}
} -body {
    set h1 [::tclpkg::cas::integrity_of_tree $d1]
    set h2 [::tclpkg::cas::integrity_of_tree $d2]
    expr {$h1 eq $h2}
} -cleanup { file delete -force -- $d1; file delete -force -- $d2 } -result 1

test cas-verify-valid "verify_integrity returns true for matching hash" -setup {
    set d [file join [::tcltest::temporaryDirectory] cas_verify]
    _make_tree $d {a.tcl "set x 1\n"}
} -body {
    set expected [::tclpkg::cas::integrity_of_tree $d]
    ::tclpkg::cas::verify_integrity $d $expected
} -cleanup { file delete -force -- $d } -result 1

test cas-verify-invalid "verify_integrity returns false for wrong hash" -setup {
    set d [file join [::tcltest::temporaryDirectory] cas_verify_bad]
    _make_tree $d {a.tcl "set x 1\n"}
} -body {
    ::tclpkg::cas::verify_integrity $d "sha256-WRONG"
} -cleanup { file delete -force -- $d } -result 0

cleanupTests
