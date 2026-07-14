# S102: Type thunking — a variable oscillates between representations
# every loop iteration
# Source: SpiceGenTcl/examples/ngspice/dc/diode_iv.tcl:22 (motivating file)
#
# C Tcl values are dual-ported (string rep + one cached intrep), so a
# value converts only when an operation REPLACES the cached intrep.  A
# loop body that alternates a self-referential variable between two
# intreps pays that replacement on every single pass: here `lappend`
# parses the string back into a list, then the interpolating `set`
# discards the list for a fresh string — two conversions per iteration.
# This is worse than S101 (a merge-point shimmer) because the cost scales
# with the iteration count, not with the number of joins.
#
# Expected: tclsh runs without error — shows the oscillation pattern.

# S102 fires: `acc` is list inside `lappend`, string after the
# interpolation, list again on the next pass's `lappend`, …
proc join_with_commas {items} {
    set acc ""
    foreach x $items {
        lappend acc $x
        set acc "$acc,"
    }
    return $acc
}
puts "oscillating: [join_with_commas {1 2 3}]"

# The fix: stay in ONE representation inside the loop and convert once at
# the end.  No S102 here — the accumulator is a list on every pass (the
# initial empty-string→list promotion on the first pass is a one-time
# cost, not thunking).
proc join_with_commas_clean {items} {
    set parts [list]
    foreach x $items {
        lappend parts $x
    }
    return "[join $parts ,],"
}
puts "clean:       [join_with_commas_clean {1 2 3}]"

puts "S102 test complete"
