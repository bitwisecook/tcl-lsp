# Issue #976: a call dispatched through a variable (`$cmd dev`) reaches
# `helper` with a literal no other call site passes, but the interprocedural
# param-constant seed enumerated only *literal* command words, so the
# dispatch counted neither for nor against `helper`'s `mode` — folding
# `$mode eq "prod"` to a fixed `true`. Must draw no I230.
proc helper {mode} {
    if {$mode eq "prod"} {
        set x 1
    } else {
        set x 2
    }
}
helper prod
helper prod
set cmd helper
$cmd dev
