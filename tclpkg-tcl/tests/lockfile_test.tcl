#!/usr/bin/env tclsh
# Tests for tclpkg lockfile serialisation and deserialisation.

package require Tcl 8.6
lappend auto_path [file join [file dirname [info script]] .. lib]
package require tclpkg

package require tcltest
namespace import ::tcltest::*

test lockfile-new "new_lockfile creates valid dict" -body {
    set lf [::tclpkg::lockfile::new_lockfile myapp ">=8.6"]
    list [dict get $lf name] [dict get $lf version] [dict get $lf tcl]
} -result {myapp 1 >=8.6}

test lockfile-new-has-timestamp "new_lockfile sets generated" -body {
    set lf [::tclpkg::lockfile::new_lockfile x]
    expr {[dict get $lf generated] ne ""}
} -result 1

test lockfile-serialise-valid-json "serialise produces valid JSON" -body {
    set lf [::tclpkg::lockfile::new_lockfile demo]
    dict set lf packages {}
    set text [::tclpkg::lockfile::serialise $lf]
    # Starts with { and ends with }
    expr {[string index $text 0] eq "\{" && [string trimright $text "\n"] eq [string trimright $text "\n"]}
} -result 1

test lockfile-serialise-sorted-keys "serialise sorts keys" -body {
    set lf [::tclpkg::lockfile::new_lockfile demo]
    dict set lf packages {}
    set text [::tclpkg::lockfile::serialise $lf]
    # "generated" should appear before "name" (alphabetical)
    set gen_pos [string first "generated" $text]
    set name_pos [string first "\"name\"" $text]
    expr {$gen_pos < $name_pos}
} -result 1

test lockfile-lookup-found "lookup finds package" -body {
    set lf [::tclpkg::lockfile::new_lockfile demo]
    set pkg [dict create name json version 1.0.0 source [dict create type tarball url "" subdir "" rev null] integrity "" size 0 requires {} provides {} license "" dev false]
    dict set lf packages [list $pkg]
    set found [::tclpkg::lockfile::lookup $lf json]
    dict get $found version
} -result "1.0.0"

test lockfile-lookup-missing "lookup returns empty for missing" -body {
    set lf [::tclpkg::lockfile::new_lockfile demo]
    dict set lf packages {}
    ::tclpkg::lockfile::lookup $lf nonexistent
} -result ""

test lockfile-write-read-roundtrip "write then read roundtrips" -setup {
    set tmpdir [file join [::tcltest::temporaryDirectory] lockfile_test]
    file mkdir $tmpdir
} -body {
    set lf [::tclpkg::lockfile::new_lockfile demo ">=8.6"]
    set pkg [dict create name json version 1.0.0 source [dict create type tarball url "" subdir "" rev null] integrity "" size 0 requires {} provides {} license "" dev false]
    dict set lf packages [list $pkg]
    set path [file join $tmpdir tclpkg.lock]
    ::tclpkg::lockfile::write $lf $path
    set lf2 [::tclpkg::lockfile::read_ $path]
    dict get $lf2 name
} -cleanup {
    file delete -force -- $tmpdir
} -result "demo"

cleanupTests
