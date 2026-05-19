#!/usr/bin/env tclsh
# Tests for tclpkg version parsing and comparison.

package require Tcl 8.6
lappend auto_path [file join [file dirname [info script]] .. lib]
package require tclpkg

package require tcltest
namespace import ::tcltest::*

test version-parse-basic "Parse 1.2.3" -body {
    set v [::tclpkg::version::parse "1.2.3"]
    list [dict get $v major] [dict get $v minor] [dict get $v patch]
} -result {1 2 3}

test version-parse-two-part "Parse 1.2 defaults patch to 0" -body {
    set v [::tclpkg::version::parse "1.2"]
    dict get $v patch
} -result 0

test version-parse-v-prefix "v prefix stripped" -body {
    set a [::tclpkg::version::normalise "v1.2.3"]
    set b [::tclpkg::version::normalise "1.2.3"]
    expr {$a eq $b}
} -result 1

test version-parse-tcl-alpha "Tcl-style a1" -body {
    set v [::tclpkg::version::parse "8.6.3a1"]
    list [dict get $v major] [dict get $v minor] [dict get $v patch] [expr {[dict get $v pre] ne ""}]
} -result {8 6 3 1}

test version-canonical "Canonical form" -body {
    ::tclpkg::version::normalise "v1.2"
} -result "1.2.0"

test version-canonical-alpha "Tcl alpha canonicalised" -body {
    ::tclpkg::version::normalise "8.6.3a1"
} -result "8.6.3-alpha.1"

test version-compare-major "Major precedence" -body {
    ::tclpkg::version::vcompare "2.0.0" "1.99.99"
} -result 1

test version-compare-pre-before-release "Prerelease before release" -body {
    ::tclpkg::version::vcompare "1.0.0-rc.1" "1.0.0"
} -result -1

test version-compare-alpha-before-beta "Alpha before beta" -body {
    ::tclpkg::version::vcompare "8.6.3a1" "8.6.3b1"
} -result -1

test version-compare-beta-before-rc "Beta before rc" -body {
    ::tclpkg::version::vcompare "8.6.3b2" "8.6.3rc1"
} -result -1

test version-compare-rc-before-release "RC before release" -body {
    ::tclpkg::version::vcompare "8.6.3rc1" "8.6.3"
} -result -1

test version-vmax "vmax picks highest" -body {
    ::tclpkg::version::vmax {1.0.0 1.2.3 1.1.9}
} -result "1.2.3"

test version-parse-invalid "Invalid raises error" -body {
    catch {::tclpkg::version::parse "not a version"} err
    expr {$err ne ""}
} -result 1

cleanupTests
