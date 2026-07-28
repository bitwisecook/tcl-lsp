# Issue #977 — a plain library file with NO `package provide`.  Both callers
# visible in this file pass "prod", so a single-file compilation unit seeds
# `mode` as that literal and folds the condition — even though
# issue977Main.tcl calls `issue977_helper dev`.
proc issue977_helper {mode} {
    if {$mode eq "prod"} {
        set r 1
    } else {
        set r 2
    }
    return $r
}
issue977_helper prod
issue977_helper prod
