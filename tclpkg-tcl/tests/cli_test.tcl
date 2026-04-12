#!/usr/bin/env tclsh
# Tests for tclpkg CLI dispatch and end-to-end pkg/venv commands.

package require Tcl 8.6
lappend auto_path [file join [file dirname [info script]] .. lib]
package require tclpkg

package require tcltest
namespace import ::tcltest::*

set _main [file join [file dirname [file normalize [info script]]] .. main.tcl]

# Run main.tcl in a given directory.
proc _run_in {dir args} {
    variable _main
    set saved [pwd]
    cd $dir
    set rc [catch {exec -- [info nameofexecutable] $_main {*}$args 2>@1} output]
    cd $saved
    return [list $rc $output]
}

proc _run {args} {
    variable _main
    set rc [catch {exec -- [info nameofexecutable] $_main {*}$args 2>@1} output]
    return [list $rc $output]
}

# Dispatch tests.

test cli-help "main --help exits cleanly" -body {
    lassign [_run --help] rc output
    expr {$rc == 0 && [string match "*pkg*" $output] && [string match "*venv*" $output]}
} -result 1

test cli-version "main --version prints version" -body {
    lassign [_run --version] rc output
    expr {$rc == 0 && [string match "*tclpkg*" $output]}
} -result 1

test cli-unknown "unknown command errors" -body {
    lassign [_run banana] rc output
    expr {$rc != 0}
} -result 1

test cli-pkg-help "pkg help lists subcommands" -body {
    lassign [_run pkg help] rc output
    expr {$rc == 0 && [string match "*init*" $output] && [string match "*install*" $output]}
} -result 1

test cli-venv-help "venv help lists subcommands" -body {
    lassign [_run venv help] rc output
    expr {$rc == 0 && [string match "*create*" $output] && [string match "*delete*" $output]}
} -result 1

# Pkg command tests.

test cli-pkg-init "pkg init creates manifest" -setup {
    set d [file join [::tcltest::temporaryDirectory] init_test]
    file mkdir $d
} -body {
    lassign [_run_in $d pkg init --name testpkg --version 0.1.0] rc output
    set mf [file join $d tclpkg.tcl]
    list $rc [file isfile $mf]
} -cleanup { file delete -force -- $d } -result {0 1}

test cli-pkg-init-content "pkg init writes correct fields" -setup {
    set d [file join [::tcltest::temporaryDirectory] init_content]
    file mkdir $d
} -body {
    _run_in $d pkg init --name mypkg --version 2.0.0
    set fd [open [file join $d tclpkg.tcl] r]
    set text [read $fd]
    close $fd
    list [string match "*mypkg*" $text] [string match "*2.0.0*" $text]
} -cleanup { file delete -force -- $d } -result {1 1}

test cli-pkg-add "pkg add appends require" -setup {
    set d [file join [::tcltest::temporaryDirectory] add_test]
    file mkdir $d
    set fd [open [file join $d tclpkg.tcl] w]
    puts $fd "package demo\nversion 1.0.0"
    close $fd
} -body {
    lassign [_run_in $d pkg add http 2.0] rc output
    set fd [open [file join $d tclpkg.tcl] r]
    set text [read $fd]
    close $fd
    list $rc [string match "*require http 2.0*" $text]
} -cleanup { file delete -force -- $d } -result {0 1}

test cli-pkg-install "pkg install creates lockfile" -setup {
    set d [file join [::tcltest::temporaryDirectory] install_test]
    file mkdir $d
    set fd [open [file join $d tclpkg.tcl] w]
    puts $fd "package demo\nversion 1.0.0\nrequire json 1.0"
    close $fd
} -body {
    lassign [_run_in $d pkg install] rc output
    list $rc [file isfile [file join $d tclpkg.lock]]
} -cleanup { file delete -force -- $d } -result {0 1}

test cli-pkg-list "pkg list shows packages" -setup {
    set d [file join [::tcltest::temporaryDirectory] list_test]
    file mkdir $d
    set fd [open [file join $d tclpkg.tcl] w]
    puts $fd "package demo\nversion 1.0.0\nrequire json 1.0"
    close $fd
    _run_in $d pkg install
} -body {
    lassign [_run_in $d pkg list] rc output
    list $rc [string match "*json*" $output]
} -cleanup { file delete -force -- $d } -result {0 1}

test cli-pkg-freeze "pkg freeze outputs require lines" -setup {
    set d [file join [::tcltest::temporaryDirectory] freeze_test]
    file mkdir $d
    set fd [open [file join $d tclpkg.tcl] w]
    puts $fd "package demo\nversion 1.0.0\nrequire json 1.0"
    close $fd
    _run_in $d pkg install
} -body {
    lassign [_run_in $d pkg freeze] rc output
    list $rc [string match "*require json*" $output]
} -cleanup { file delete -force -- $d } -result {0 1}

test cli-pkg-tree "pkg tree shows tree" -setup {
    set d [file join [::tcltest::temporaryDirectory] tree_test]
    file mkdir $d
    set fd [open [file join $d tclpkg.tcl] w]
    puts $fd "package demo\nversion 1.0.0\nrequire json 1.0"
    close $fd
    _run_in $d pkg install
} -body {
    lassign [_run_in $d pkg tree] rc output
    list $rc [string match "*demo*" $output] [string match "*json*" $output]
} -cleanup { file delete -force -- $d } -result {0 1 1}

# Venv command tests.

test cli-venv-create "venv create makes directory" -setup {
    set d [file join [::tcltest::temporaryDirectory] venv_create]
    file mkdir $d
    set venv [file join $d .venv]
} -body {
    lassign [_run venv create $venv] rc output
    list $rc [file isdir $venv] [file isfile [file join $venv tclvenv.cfg]]
} -cleanup { file delete -force -- $d } -result {0 1 1}

test cli-venv-info "venv info shows config" -setup {
    set d [file join [::tcltest::temporaryDirectory] venv_info]
    file mkdir $d
    set venv [file join $d .venv]
    _run venv create $venv
} -body {
    lassign [_run venv info $venv] rc output
    list $rc [string match "*tcl_version*" $output]
} -cleanup { file delete -force -- $d } -result {0 1}

test cli-venv-delete "venv delete removes directory" -setup {
    set d [file join [::tcltest::temporaryDirectory] venv_del]
    file mkdir $d
    set venv [file join $d .venv]
    _run venv create $venv
} -body {
    lassign [_run venv delete $venv] rc output
    list $rc [file exists $venv]
} -cleanup { file delete -force -- $d } -result {0 0}

cleanupTests
