# S100/S101 shimmer precision fixture: TP/FP/TN/FN cases from the deep
# shimmer review (interp alias, TclOO, namespace eval, write-traces,
# suppression directives).

# True case: a committed dict intrep gets shimmered to a list by lindex.
proc shimmer_true_case {} {
    set x [dict create a 1 b 2]
    lindex $x 0
}

# False case: the value already carries a list intrep, so no shimmer.
proc shimmer_false_case {} {
    set y [list 1 2 3]
    lindex $y 0
}

# True case: shimmer fires through an interp alias to a shimmering builtin.
interp alias {} myindex {} ::lindex
proc shimmer_through_alias {} {
    set z [dict create a 1 b 2]
    myindex $z 0
}

# True case: shimmer fires inside a TclOO method body.
oo::class create ShimmerClass {
    method touch {} {
        set a hello
        incr a
    }
}

# True case: shimmer fires inside a namespace eval body.
namespace eval shimmerNs {
    set b hello
    incr b
}

# False case (FN guard): a write-traced variable is never confidently typed,
# so shimmer must not fire even though the RHS looks stringy.
proc shimmer_traced_var {} {
    trace add variable c write cb
    set c hello
    incr c
}

# Suppression directive silences the diagnostic on the following line.
proc shimmer_suppress_directive_matches {} {
    set d [dict create a 1 b 2]
    # noqa: S100
    lindex $d 0
}

# A directive naming a different code must not silence this one.
proc shimmer_suppress_directive_mismatches {} {
    set e [dict create a 1 b 2]
    # noqa: W100
    lindex $e 0
}
