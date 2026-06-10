#!/usr/bin/env tclsh
# reference_test_runner.tcl — Run a single .test file, capture structured results.
#
# Usage: tclsh reference_test_runner.tcl <tests_dir> <test_file>
#
# Output (to stdout):
#
#   TCLSH    /usr/local/bin/tclsh9.0
#   VERSION  9.0.3
#   FILE     compExpr-old.test
#   DATE     2026-03-03T12:34:56Z
#   ---
#   PASS compExpr-old-1.1
#   SKIP compExpr-old-5.1 testexprlong
#   FAIL compExpr-old-99.1
#   ---
#   TOTAL   184
#   PASSED  184
#   SKIPPED 0
#   FAILED  0
#
# The per-test lines (PASS / SKIP / FAIL) are extracted by hooking
# tcltest's verbose output via a reflected channel on outputChannel.

if {$argc < 2} {
    puts stderr "Usage: tclsh reference_test_runner.tcl <tests_dir> <test_file>"
    exit 1
}

set tests_dir [lindex $argv 0]
set test_file [lindex $argv 1]

# Override argv BEFORE loading tcltest — tcltest processes ::argv
# during package require and would choke on our tests_dir path.
set ::argv [list -verbose {pass skip fail error}]
set ::argc 2

cd $tests_dir

# Header
puts "TCLSH    [info nameofexecutable]"
puts "VERSION  [info patchlevel]"
puts "FILE     $test_file"
puts "DATE     [clock format [clock seconds] -format %Y-%m-%dT%H:%M:%SZ -gmt 1]"
puts "---"

# Load tcltest
package require tcltest 2.5
namespace import -force ::tcltest::*

# Try to load C-level test commands (available in test builds).
# Plain tclsh won't have these — tests gated by constraints will
# be properly skipped.
catch {::tcltest::loadTestedCommands}
catch {package require -exact tcl::test [info patchlevel]}
catch {package require -exact Tcltest  [info patchlevel]}

# Hook: capture per-test outcomes
# We intercept the output channel to parse tcltest's verbose lines.
# Configure verbose to include pass, skip, fail, and error.
::tcltest::configure -verbose {pass skip fail error}

# Redirect tcltest output through a custom channel that parses
# verbose lines and emits our structured format.
#
# tcltest verbose formats (from tcltest.tcl):
#   pass:  "++++ <name> PASSED"
#   skip:  "++++ <name> SKIPPED: <constraints>"
#   fail:  "==== <name> <description> FAILED ===="
#          ... details ...
#          "==== <name> FAILED"

# We'll collect results by intercepting puts on the output channel.
set ::_ref_pass  [list]
set ::_ref_skip  [list]
set ::_ref_fail  [list]
set ::_ref_skip_reason [dict create]

# Build a custom channel using a reflected channel (Tcl 8.5+).
# The namespace ensemble makes ::refchan callable as a command
# so chan create can dispatch ::refchan initialize/write/etc.
namespace eval ::refchan {
    namespace export initialize finalize watch write
    namespace ensemble create
    variable buf ""
}

proc ::refchan::initialize {id mode} {
    return {initialize finalize watch write}
}

proc ::refchan::finalize {id} {}

proc ::refchan::watch {id events} {}

proc ::refchan::write {id data} {
    variable buf
    append buf $data

    # Process complete lines
    while {[set idx [string first "\n" $buf]] >= 0} {
        set line [string range $buf 0 $idx-1]
        set buf [string range $buf $idx+1 end]
        _process_line $line
    }
    return [string length $data]
}

proc ::refchan::_process_line {line} {
    # Pass: "++++ <name> PASSED"
    if {[regexp {^\+\+\+\+ (.+?) PASSED$} $line -> name]} {
        lappend ::_ref_pass $name
        return
    }
    # Skip: "++++ <name> SKIPPED: <reason>"
    if {[regexp {^\+\+\+\+ (.+?) SKIPPED: (.*)$} $line -> name reason]} {
        lappend ::_ref_skip $name
        dict set ::_ref_skip_reason $name $reason
        return
    }
    # Fail end marker: "==== <name> FAILED"
    # (This is the closing line; the opening line has a description.)
    if {[regexp {^==== (.+?) FAILED$} $line -> name]} {
        # Avoid duplicates (opening line also matches a different pattern)
        if {$name ni $::_ref_fail} {
            lappend ::_ref_fail $name
        }
        return
    }
}

# Create the custom channel and redirect tcltest output.
# We set ::tcltest::outputChannel directly because -outfile expects
# a filename, not a channel handle.
set ::_refchan [chan create write ::refchan]
set ::tcltest::outputChannel $::_refchan

# Run the test file
if {[catch {source [file join $tests_dir $test_file]} err]} {
    puts stderr "ERROR sourcing $test_file: $err"
}

# Flush any partial buffer in the reflected channel
if {$::refchan::buf ne ""} {
    ::refchan::_process_line $::refchan::buf
    set ::refchan::buf ""
}

# Output per-test results
foreach name [lsort $::_ref_pass] {
    puts "PASS $name"
}
foreach name [lsort $::_ref_skip] {
    set reason ""
    catch {set reason [dict get $::_ref_skip_reason $name]}
    puts "SKIP $name $reason"
}
foreach name [lsort $::_ref_fail] {
    puts "FAIL $name"
}

# Output summary
puts "---"

# Always use our captured lists for counts — cleanupTests (called at
# the end of most test files) resets ::tcltest::numTests to zero.
set total [expr {[llength $::_ref_pass] + [llength $::_ref_skip] + [llength $::_ref_fail]}]
puts "TOTAL   $total"
puts "PASSED  [llength $::_ref_pass]"
puts "SKIPPED [llength $::_ref_skip]"
puts "FAILED  [llength $::_ref_fail]"
