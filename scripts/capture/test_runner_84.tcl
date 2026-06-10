#!/usr/bin/env tclsh
# reference_test_runner_84.tcl — Run a single .test file in Tcl 8.4.
#
# Tcl 8.4 has tcltest (2.2) but lacks reflected channels (chan create),
# dict, ni, and {*}, so this helper uses temp files instead of a
# reflected channel and arrays instead of dict.
#
# Usage: tclsh8.4 reference_test_runner_84.tcl <tests_dir> <test_file>
#
# Output format matches reference_test_runner.tcl:
#
#   TCLSH    /path/to/tclsh8.4
#   VERSION  8.4.20
#   FILE     compExpr-old.test
#   DATE     2026-03-03T12:34:56
#   ---
#   PASS compExpr-old-1.1
#   SKIP compExpr-old-5.1 testexprlong
#   FAIL compExpr-old-99.1
#   ---
#   TOTAL   184
#   PASSED  184
#   SKIPPED 0
#   FAILED  0

proc main {} {
    global argv argc

    if {$argc < 2} {
        puts stderr "Usage: tclsh reference_test_runner_84.tcl <tests_dir> <test_file>"
        exit 1
    }

    set tests_dir [lindex $argv 0]
    set test_file [lindex $argv 1]

    # Override argv BEFORE loading tcltest — tcltest processes ::argv
    # during package require and would choke on our tests_dir path.
    # 8.4 uses [eval configure $argv] not [configure {*}$argv].
    set ::argv [list -verbose {pass skip fail error}]
    set ::argc 2

    cd $tests_dir

    # Header
    puts "TCLSH    [info nameofexecutable]"
    puts "VERSION  [info patchlevel]"
    puts "FILE     $test_file"
    puts "DATE     [clock format [clock seconds] -format %Y-%m-%dT%H:%M:%S]"
    puts "---"

    # Load tcltest
    # Tcl 8.4 ships tcltest 2.2; don't require 2.5
    package require tcltest
    namespace import -force ::tcltest::*

    # Try to load C-level test commands
    catch {::tcltest::loadTestedCommands}
    catch {package require Tcltest [info patchlevel]}

    # Track per-test results
    # We can't use reflected channels (8.6+) or dict (8.5+).
    # Instead, redirect tcltest output to a temp file, then parse it.

    set ::_ref_pass [list]
    set ::_ref_skip [list]
    set ::_ref_fail [list]
    # Use arrays instead of dict for skip reasons
    array set ::_ref_skip_reason {}

    # Configure verbose: show pass, skip, fail output
    configure -verbose {pass skip fail error}

    # Redirect tcltest output to a temp file.  We set outputChannel
    # directly because -outfile expects a filename, not a channel.
    set tmpdir "/tmp"
    catch {set tmpdir $::env(TMPDIR)}
    set ::_ref_outfile "$tmpdir/tclref_[pid].out"
    set ::_ref_chan [open $::_ref_outfile w]
    set ::tcltest::outputChannel $::_ref_chan

    # Run the test file
    if {[catch {source [file join $tests_dir $test_file]} err]} {
        puts stderr "ERROR sourcing $test_file: $err"
    }

    # Parse the captured output
    close $::_ref_chan

    set fd [open $::_ref_outfile r]
    set content [read $fd]
    close $fd
    file delete $::_ref_outfile

    foreach line [split $content "\n"] {
        # Pass: "++++ <name> PASSED"
        if {[regexp {^\+\+\+\+ (.+?) PASSED$} $line -> name]} {
            lappend ::_ref_pass $name
            continue
        }
        # Skip: "++++ <name> SKIPPED: <reason>"
        if {[regexp {^\+\+\+\+ (.+?) SKIPPED: (.*)$} $line -> name reason]} {
            lappend ::_ref_skip $name
            set ::_ref_skip_reason($name) $reason
            continue
        }
        # Fail end marker: "==== <name> FAILED"
        if {[regexp {^==== (.+?) FAILED$} $line -> name]} {
            if {[lsearch -exact $::_ref_fail $name] < 0} {
                lappend ::_ref_fail $name
            }
            continue
        }
    }

    # Output per-test results
    foreach name [lsort $::_ref_pass] {
        puts "PASS $name"
    }
    foreach name [lsort $::_ref_skip] {
        set reason ""
        catch {set reason $::_ref_skip_reason($name)}
        puts "SKIP $name $reason"
    }
    foreach name [lsort $::_ref_fail] {
        puts "FAIL $name"
    }

    # Output summary
    puts "---"

    # Always use our captured lists — cleanupTests resets numTests.
    set total [expr {[llength $::_ref_pass] + [llength $::_ref_skip] + [llength $::_ref_fail]}]
    puts "TOTAL   $total"
    puts "PASSED  [llength $::_ref_pass]"
    puts "SKIPPED [llength $::_ref_skip]"
    puts "FAILED  [llength $::_ref_fail]"
}

main
