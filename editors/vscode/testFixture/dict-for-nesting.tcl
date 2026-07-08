# Issue #833 regression fixture.
#
# Reads nested inside a `dict for` body must keep their feeding store live: here
# `$x` is read as the command name of `$x a $key`, inside `if` inside `dict for`,
# so `set x set` is NOT a dead store (no W220 / W211 on `x`).
proc issue_833_clean {} {
    set x set
    dict for {key value} [dict create a true b false] {
        if {$value} {
            $x a $key
        }
    }
}

# A genuine dead store *inside* the (now-lowered) dict-for body still fires W220 —
# which also proves the analyser walked into the body's own control flow.
proc dict_for_body_dead_store {d} {
    dict for {k v} $d {
        set tmp 1
        set tmp 2
        puts $tmp
    }
}
