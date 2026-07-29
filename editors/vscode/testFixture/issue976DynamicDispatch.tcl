# Issue #976: a call dispatched through a variable (`$cmd dev`) reaches
# `i976dispatchHelper` with a literal no other call site passes, but the interprocedural
# param-constant seed enumerated only *literal* command words, so the
# dispatch counted neither for nor against `i976dispatchHelper`'s `mode` — folding
# `$mode eq "prod"` to a fixed `true`. Must draw no I230.
proc i976dispatchHelper {mode} {
    if {$mode eq "prod"} {
        set x 1
    } else {
        set x 2
    }
}
i976dispatchHelper prod
i976dispatchHelper prod
set cmd i976dispatchHelper
$cmd dev
