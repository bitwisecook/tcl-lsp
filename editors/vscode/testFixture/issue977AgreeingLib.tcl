# Issue #977 TP control — the same library shape whose project-wide callers
# all agree on the literal, so the interprocedural seed is sound and I230
# must still fire.
proc agreeing_helper {mode} {
    if {$mode eq "prod"} {
        set r 1
    } else {
        set r 2
    }
    return $r
}
agreeing_helper prod
agreeing_helper prod
