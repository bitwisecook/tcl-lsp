# Fixture for the `expr` math-function completion + hover tests (issue #974).
# Line 3 (0-indexed) ends in a deliberately unfinished `expr` body: the cursor
# after `si` sits inside an EXPR-role argument, where the bare math functions
# are the only callable candidates.
proc sizzler {} { return 1 }
set finished [expr {sin(1.0)}]
set unfinished [expr {si
