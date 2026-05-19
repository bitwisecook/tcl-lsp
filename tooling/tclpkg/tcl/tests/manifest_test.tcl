#!/usr/bin/env tclsh
# Tests for tclpkg manifest loading.

package require Tcl 8.6
lappend auto_path [file join [file dirname [info script]] .. lib]
package require tclpkg

package require tcltest
namespace import ::tcltest::*

test manifest-minimal "Minimal manifest parses" -body {
    set ast [::tclpkg::manifest::load_text "package myapp\nversion 1.0.0\n"]
    list [dict get $ast name] [dict get $ast version]
} -result {myapp 1.0.0}

test manifest-default-tcl "Default tcl constraint" -body {
    set ast [::tclpkg::manifest::load_text "package a\nversion 1.0.0\n"]
    dict get $ast tcl_constraint
} -result ">=8.6"

test manifest-require "Simple require" -body {
    set ast [::tclpkg::manifest::load_text "package a\nversion 1.0.0\nrequire json 1.3.5\n"]
    set reqs [dict get $ast requires]
    dict get [lindex $reqs 0] name
} -result "json"

test manifest-dev-require "dev-require" -body {
    set ast [::tclpkg::manifest::load_text "package a\nversion 1.0.0\ndev-require tcltest 2.5\n"]
    set reqs [dict get $ast dev_requires]
    dict get [lindex $reqs 0] name
} -result "tcltest"

test manifest-missing-package "Missing package raises" -body {
    catch {::tclpkg::manifest::load_text "version 1.0.0\n"} err
    string match "*missing*package*" $err
} -result 1

test manifest-missing-version "Missing version raises" -body {
    catch {::tclpkg::manifest::load_text "package a\n"} err
    string match "*missing*version*" $err
} -result 1

test manifest-safe-blocks-exec "exec is blocked" -body {
    catch {::tclpkg::manifest::load_text "package a\nversion 1.0.0\nexec ls\n"} err
    expr {$err ne ""}
} -result 1

test manifest-metadata "Description and license" -body {
    set ast [::tclpkg::manifest::load_text "package a\nversion 1.0.0\ndescription {Hello}\nlicense MIT\n"]
    list [dict get $ast description] [dict get $ast license]
} -result {Hello MIT}

test manifest-replace "Replace directive" -body {
    set ast [::tclpkg::manifest::load_text "package a\nversion 1.0.0\nreplace json 1.4.0 https://x.org/j.tar.gz\n"]
    set reps [dict get $ast replaces]
    dict get [lindex $reps 0] name
} -result "json"

test manifest-exclude "Exclude directive" -body {
    set ast [::tclpkg::manifest::load_text "package a\nversion 1.0.0\nexclude http 2.9.7\n"]
    set excs [dict get $ast excludes]
    dict get [lindex $excs 0] name
} -result "http"

cleanupTests
