# Fixture for the S101 (shimmer inside loop) deep-review VS Code extension
# tests. Covers: a true-positive per-iteration shimmer in a plain proc, the
# same shape inside a TclOO method body (methods previously fell through a
# fresh-rebuild-only fix — see tcl-lsp-db's proc_taint_solve), a guarded
# false positive via `my variable` (an object-instance variable's real
# intrep depends on another method's last write, not the nominal return
# type of `my`), and a numeric eq/ne quick fix (S100) on a sibling shimmer
# check that landed in the same review.

proc shimmerLoop {items} {
    foreach x $items {
        set y [lindex $x 0]
    }
}

proc numericEqShimmer {} {
    set n 42
    set z [expr {$n eq 42}]
    return $z
}

oo::class create Counter {
    variable count

    constructor {} {
        set count 0
    }

    method bump {} {
        my variable count
        incr count
    }

    method scan {items} {
        foreach x $items {
            set y [lindex $x 0]
        }
    }
}
