# Fixture for the O101 (fold constant integer expressions) VS Code
# extension test — covers one true-positive fold and one guarded
# false-positive (a user-defined ::tcl::mathfunc:: shadows the builtin).

proc ::tcl::mathfunc::triple {x} {
    return [expr {$x * 3}]
}

proc guarded {} {
    # `triple` is shadowed above, so O101 must NOT fold this call —
    # folding it would assume the builtin math function, not the
    # user-defined override.
    return [expr {triple(2)}]
}

proc unguarded {} {
    # An ordinary constant expression, unaffected by the mathfunc
    # shadowing above (the shadow gate is name-specific, not
    # whole-module) — O101 folds this to `return 3`.
    return [expr {1 + 2}]
}
