proc demonstrate {arg1 arg2 arg3 arg4 arg5 arg6 arg7} {
    return "$arg1 $arg2 $arg3 $arg4 $arg5 $arg6 $arg7"
}

# Too many arguments for the same-file proc — must fire E003 (this used to
# produce no diagnostic at all, even though an out-of-range variable
# reference inside the proc body correctly fires W210).
demonstrate one two three four five six seven eight

# A correctly-arg-counted call to a different same-file proc must stay
# silent.
proc need3 {a b c} {
    return "$a$b$c"
}
need3 1 2 3

# `forward NAME my TARGET ?ARG…?` — the TclOO idiom for forwarding to a
# sibling method — must be arity-checked shifted by the prepended args.
oo::class create Widget {
    method base {a b c} {
        return "$a$b$c"
    }
    forward fwd my base fixedarg
}
set w1 [Widget new]
$w1 fwd 1 2 3
