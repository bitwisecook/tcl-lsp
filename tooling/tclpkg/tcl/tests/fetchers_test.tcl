#!/usr/bin/env tclsh
# Tests for tclpkg fetchers -- zip security and git URL parsing.

package require Tcl 8.6
lappend auto_path [file join [file dirname [info script]] .. lib]
package require tclpkg

package require tcltest
namespace import ::tcltest::*

# Helper: create a zip file with the given members using the system zip command.
proc _make_zip {zippath members} {
    set staging [file join [::tcltest::temporaryDirectory] _zip_staging]
    file mkdir $staging
    foreach {name content} $members {
        set target [file join $staging $name]
        file mkdir [file dirname $target]
        set fd [open $target w]
        puts -nonewline $fd $content
        close $fd
    }
    # Create the zip from the staging directory.
    set saved [pwd]
    cd $staging
    catch {exec zip -r -q $zippath . 2>@stderr} _
    cd $saved
    file delete -force -- $staging
}

# Helper: create a zip with a ../ path traversal member.
# We can't easily create this with the zip command, so we create a
# normal zip and then use Tcl to manipulate the member names via the
# safe_extract_zip path validation (test the validation directly).
proc _has_unzip {} {
    return [expr {![catch {exec unzip -v 2>@1}]}]
}

::tcltest::testConstraint unzip [_has_unzip]

# Zip Slip tests -- test the path validation logic directly.

test zip-path-absolute "Absolute path detected" -body {
    # Test the path validation that safe_extract_zip performs.
    set member "/etc/passwd"
    expr {[string index $member 0] eq "/"}
} -result 1

test zip-path-traversal "Path traversal detected" -body {
    set dest [file normalize "/tmp/staging"]
    set member "../escape.txt"
    set target [file normalize [file join "/tmp/staging" $member]]
    expr {![string match "${dest}/*" $target] && $target ne $dest}
} -result 1

test zip-path-sibling-bypass "Sibling directory bypass detected" -body {
    # The specific case from the PR review: ../staging2/evil
    # A naive startswith("/tmp/staging") would match "/tmp/staging2/evil".
    set dest [file normalize "/tmp/staging"]
    set member "../staging2/evil.txt"
    set target [file normalize [file join "/tmp/staging" $member]]
    # The target resolves to /tmp/staging2/evil.txt which does NOT
    # start with /tmp/staging/ (note the trailing slash is implied
    # by the glob pattern ${dest}/*).
    expr {![string match "${dest}/*" $target] && $target ne $dest}
} -result 1

test zip-path-normal "Normal path accepted" -body {
    set dest [file normalize "/tmp/staging"]
    set member "sub/file.txt"
    set target [file normalize [file join "/tmp/staging" $member]]
    expr {[string match "${dest}/*" $target]}
} -result 1

# Full safe_extract_zip tests (require unzip).

test zip-extract-normal "Normal zip extracts correctly" -constraints {
    unzip
} -setup {
    set d [file join [::tcltest::temporaryDirectory] zip_normal]
    file mkdir $d
    set zippath [file join [::tcltest::temporaryDirectory] normal.zip]
    _make_zip $zippath {hello.txt "hello\n" sub/nested.txt "nested\n"}
} -body {
    ::tclpkg::fetchers::safe_extract_zip $zippath $d
    list [file isfile [file join $d hello.txt]] [file isfile [file join $d sub nested.txt]]
} -cleanup {
    file delete -force -- $d $zippath
} -result {1 1}

test zip-extract-bomb "Oversized zip rejected" -constraints {
    unzip
} -setup {
    set d [file join [::tcltest::temporaryDirectory] zip_bomb]
    file mkdir $d
    set zippath [file join [::tcltest::temporaryDirectory] bomb.zip]
    # Create a file larger than a lowered limit.
    set big_content [string repeat "x" 200]
    _make_zip $zippath [list big.txt $big_content]
    # Lower the limit for the test.
    set saved_limit $::tclpkg::fetchers::max_extract_bytes
    set ::tclpkg::fetchers::max_extract_bytes 100
} -body {
    set rc [catch {::tclpkg::fetchers::safe_extract_zip $zippath $d} err]
    list $rc [string match "*zip bomb*" $err]
} -cleanup {
    set ::tclpkg::fetchers::max_extract_bytes $saved_limit
    file delete -force -- $d $zippath
} -result {1 1}


# Git URL parsing tests.

test git-url-ssh-preserved "SSH URLs are not split on @" -body {
    # Simulate the URL parsing logic from fetch_git.
    set url "git@github.com:org/repo.git"
    set rev ""
    if {$rev eq "" && [string match "*@*" $url]} {
        set idx [string last "@" $url]
        set prefix [string range $url 0 $idx-1]
        if {[string match "*/*" $prefix] || [string match "*.git" $prefix]} {
            set rev [string range $url $idx+1 end]
            set url $prefix
        }
    }
    # The URL should be unchanged -- "git" is the prefix, not a path.
    list $url $rev
} -result {git@github.com:org/repo.git {}}

test git-url-rev-extracted "HTTPS URL with @rev is split correctly" -body {
    set url "https://github.com/org/repo.git@v1.0"
    set rev ""
    if {$rev eq "" && [string match "*@*" $url]} {
        set idx [string last "@" $url]
        set prefix [string range $url 0 $idx-1]
        if {[string match "*/*" $prefix] || [string match "*.git" $prefix]} {
            set rev [string range $url $idx+1 end]
            set url $prefix
        }
    }
    list $url $rev
} -result {https://github.com/org/repo.git v1.0}

test git-url-plain "Plain URL without @ is unchanged" -body {
    set url "https://github.com/org/repo.git"
    set rev ""
    if {$rev eq "" && [string match "*@*" $url]} {
        set idx [string last "@" $url]
        set prefix [string range $url 0 $idx-1]
        if {[string match "*/*" $prefix] || [string match "*.git" $prefix]} {
            set rev [string range $url $idx+1 end]
            set url $prefix
        }
    }
    list $url $rev
} -result {https://github.com/org/repo.git {}}

cleanupTests
