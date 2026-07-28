# Issue #977 — a plain library file with no `package provide`.  Both callers
# visible here pass "prod", so a single-file compilation folds the condition.
proc issue977_helper {mode} {
    if {$mode eq "prod"} { set r 1 } else { set r 2 }
    return $r
}
issue977_helper prod
issue977_helper prod
