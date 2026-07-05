# Extract the reg.test corpus into a flat, typed, tab-delimited form by driving
# the REAL Tcl 9 engine (the `testregexp` command, available in the `tcltest`
# shell) as the oracle.  Run with:
#
#   tmp/tcl9.0.3/unix/tcltest rust/tcl-regex/tests/data/dump_reg.tcl
#
# We reproduce reg.test's `&` ARE/BRE expansion and its flag-string -> option
# mapping (TestFlags), then ask the engine itself for the authoritative
# `-about` info (nsub + re_info bits) and the `-indices` submatch positions.
# Because reg.test's own assertions are against this very engine, matching this
# data is exactly equivalent to passing reg.test.
#
# Output: one record per final variant, tab separated:
#   id  flags  reCp  hasTarget  targetCp  status  nsub  info  matched  pairs
# where:
#   reCp/targetCp  comma-separated decimal codepoints
#   status         "error:<NAME>" | "ok"
#   nsub           subexpression count (empty for error)
#   info           comma-separated REG_* info-bit names (in bit order)
#   matched        1 / 0          (empty when no target)
#   pairs          comma list of so:eo inclusive-end index pairs (per group)

proc cps {s} {
    set r {}
    foreach ch [split $s ""] {
        lappend r [scan $ch %c]
    }
    return [join $r ,]
}

# reg.test's TestFlags: map a flag string to testregexp option arguments.
proc TestFlags {fl} {
    set args [list]
    set flags ""
    foreach f [split $fl ""] {
        switch -exact -- $f {
            "i" { lappend args "-nocase" }
            "x" { lappend args "-expanded" }
            "n" { lappend args "-line" }
            "p" { lappend args "-linestop" }
            "w" { lappend args "-lineanchor" }
            "-" { }
            default { append flags $f }
        }
    }
    if {$flags ne ""} {
        lappend args -xflags $flags
    }
    return $args
}

set ::bug 0

proc emit {id flags re hasTarget target} {
    set opts [TestFlags $flags]
    # Compile + -about.
    if {[catch {testregexp -about {*}$opts $re} about]} {
        set name [lindex $::errorCode 1]
        puts [join [list $id $flags [cps $re] $hasTarget \
                [expr {$hasTarget ? [cps $target] : ""}] "error:$name" "" "" "" ""] \t]
        return
    }
    set nsub [lindex $about 0]
    set info [join [lindex $about 1] ,]
    set matched ""
    set pairs ""
    if {$hasTarget} {
        # REG_EXPECT (the `t` flag) makes testregexp write a final extra var
        # holding the internal DFA coldspot (rm_extend); provision it so the
        # real match/submatch slots are not clobbered, but do not record it —
        # it is an incremental-match artifact, not user-facing ARE behaviour.
        set expect [string match *t* $flags]
        set nvars [expr {$nsub + 1 + ($expect ? 1 : 0)}]
        set names [list]
        for {set i 0} {$i < $nvars} {incr i} {
            lappend names m$i
            set m$i {}
        }
        set matched [eval [list testregexp -indices {*}$opts $re $target] $names]
        if {$matched} {
            set pl [list]
            for {set i 0} {$i <= $nsub} {incr i} {
                set v [set m$i]
                lappend pl "[lindex $v 0]:[lindex $v 1]"
            }
            set pairs [join $pl ,]
        }
    }
    puts [join [list $id $flags [cps $re] $hasTarget \
            [expr {$hasTarget ? [cps $target] : ""}] ok $nsub $info $matched $pairs] \t]
}

# `&` ARE/BRE expansion shared by every directive type.
proc expand {id flags re hasTarget target} {
    if {[string match *&* $flags]} {
        set f [string map {& {}} $flags]
        expand "$id ARE" $f $re $hasTarget $target
        expand "$id BRE" "${f}b" $re $hasTarget $target
        return
    }
    if {$::bug || [string match *+* $flags]} {
        return
    }
    emit $id $flags $re $hasTarget $target
}

proc expectMatch {id flags re target args}   { expand $id $flags $re 1 $target }
proc expectNomatch {id flags re target args} { expand $id $flags $re 1 $target }
proc expectIndices {id flags re target args} { expand $id $flags $re 1 $target }
proc expectPartial {id flags re target args} { expand $id $flags $re 1 $target }
proc expectError {id flags re err}           { expand $id $flags $re 0 {} }
proc knownBug {args} { set ::bug 1; uplevel 1 $args; set ::bug 0 }
proc doing {args} {}

set here [file dirname [info script]]
set fh [open [file join $here reg.test] r]
fconfigure $fh -encoding utf-8
set body [read $fh]
close $fh
set marker "######## the tests themselves ########"
set idx [string first $marker $body]
set data [string range $body [expr {$idx + [string length $marker]}] end]
namespace eval ::tcltest {}
proc ::tcltest::test {args} {}
proc test {args} {}
proc ::tcltest::testConstraint {args} {}
proc ::tcltest::cleanupTests {args} {}
eval $data
