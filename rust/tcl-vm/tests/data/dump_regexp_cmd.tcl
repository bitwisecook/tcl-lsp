# Extract the regexp.test / regexpComp.test command-level corpus by overriding
# tcltest's `test` to emit each case's (name, constraints, returnCodes, result,
# body) instead of running it. Run under the `tcltest` shell:
#
#   tmp/tcl9.0.3/unix/tcltest \
#       rust/tcl-vm/tests/data/dump_regexp_cmd.tcl <file.test> > out.tsv
#
# Strings (result + body) are emitted as comma-separated codepoints. The Rust
# side runs each body through the VM and compares the result; a documented
# skip-list covers cases that need test scaffolding the VM does not provide
# (NOT engine differences).

proc ::cps {s} {
    set r {}
    foreach ch [split $s ""] {
        lappend r [scan $ch %c]
    }
    return [join $r ,]
}

# Capture the target file, then clear argv so tcltest's own argv parser
# (invoked by `package require tcltest`) doesn't choke on it.
set ::dumpfile [lindex $::argv 0]
set ::argv {}
set ::argc 0

package require tcltest

namespace eval ::tcltest {
    rename test __orig_test
    proc test {name desc args} {
        set body ""
        set result ""
        set constraints ""
        set rcodes "ok"
        if {[llength $args] == 2} {
            set body [lindex $args 0]
            set result [lindex $args 1]
        } else {
            foreach {k v} $args {
                switch -- $k {
                    -body { set body $v }
                    -result { set result $v }
                    -constraints { set constraints $v }
                    -returnCodes { set rcodes $v }
                }
            }
        }
        puts [join [list $name $constraints $rcodes [::cps $result] [::cps $body]] \t]
    }
    # Avoid the end-of-file summary/exit; everything else (constraints,
    # loadTestedCommands, configure) uses the real framework.
    proc cleanupTests {args} {}
}

source $::dumpfile
