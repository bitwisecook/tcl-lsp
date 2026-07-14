# S101: Shimmer — paths carrying different internal representations merge
# Source: SpiceGenTcl/examples/ngspice/dc/diode_iv.tcl:30 (motivating file)
#
# In C Tcl every value is a Tcl_Obj holding a string representation and at
# most ONE cached internal representation (intrep) — list, dict, int, …
# The two are dual-ported: generating the string rep of a list (e.g.
# interpolating "$xs" into a message) KEEPS the list intrep, so reading a
# value both ways is not, by itself, a shimmer.  A shimmer happens when an
# operation must REPLACE the cached intrep — using a string-built value as
# a list forces a parse; using a list-built value as a dict rebuilds the
# hash — and the old intrep is discarded.
#
# S101 fires when two control-flow paths assign DIFFERENT intreps to the
# same variable and converge inside a loop: past the join every typed use
# must be prepared to convert, and the conversion repeats each iteration.
# (The same merge outside a loop is a one-time conversion — S100.)
#
# Expected: tclsh runs without error — demonstrates the merge pattern.

# The separator is built as a LIST on one arm and a STRING on the other.
# Past the join, [lindex $sep 0] must re-parse the string arm's value on
# every iteration that took the else branch — S101 fires at the join (and
# at the loop-carried merge with the pre-loop initialiser).
proc render {rows fancy} {
    set sep ""
    foreach row $rows {
        if {$fancy} {
            set sep [list "|" "-"]
        } else {
            set sep "--"
        }
        puts "[lindex $sep 0] $row"
    }
}
render {a b} 1
render {c d} 0

# The fix: build the SAME representation on both arms, so the join carries
# one intrep and the list use never re-parses.  No S101 here.
proc render_clean {rows fancy} {
    foreach row $rows {
        if {$fancy} {
            set sep [list "|" "-"]
        } else {
            set sep [list "--"]
        }
        puts "[lindex $sep 0] $row"
    }
}
render_clean {a b} 1

# NOT a shimmer (dual-ported reads): building a list and *reading* it as a
# string only generates the string rep alongside the intact list intrep —
# the later lindex/llength calls reuse the cached list without re-parsing.
set xydata {}
foreach x {1.0 2.0 3.0} y {0.5 1.5 2.5} {
    lappend xydata [list [format "%.3f" $x] [format "%.3f" $y]]
}
puts "as string: $xydata"
puts "first element: [lindex $xydata 0]"
puts "length: [llength $xydata]"

puts "S101 test complete"
