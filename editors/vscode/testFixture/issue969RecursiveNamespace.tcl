# Issue #969: "Condition '$count & 1' is always false" fired on a genuinely
# alternating parity check inside a recursive, namespaced proc. Root cause:
# the interprocedural param-constant seed resolved `dfs`'s bare recursive
# self-call against the global namespace instead of `::graph`, so that call
# site (with its necessarily-varying argument) silently vanished from the
# evidence and only the one external caller's literal `0` remained — folding
# `$count & 1` to a fixed `false`. Must draw no I230.
namespace eval ::graph {
    proc dfs {count} {
        if {$count & 1} {
            set parity odd
        } else {
            set parity even
        }
        if {$count < 3} {
            dfs [expr {$count + 1}]
        }
    }
}
::graph::dfs 0
