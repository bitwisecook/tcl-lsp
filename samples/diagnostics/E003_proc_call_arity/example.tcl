# E003: Too many arguments for a same-file proc / alias / rename / TclOO
# method call
# Source: user-reported interactive session — calling a same-file proc
# with the wrong number of arguments produced no diagnostic at all, even
# though an out-of-range variable reference inside the proc body
# correctly fired W210.
#
# Expected: tclsh raises "wrong # args: should be "demonstrate arg1 arg2
# arg3 arg4 arg5 arg6 arg7"" for Case 1, and runs cleanly for Cases 2-4.

# Case 1: same-file proc called with one too many arguments (true positive)
proc demonstrate {arg1 arg2 arg3 arg4 arg5 arg6 arg7} {
    return "$arg1 $arg2 $arg3 $arg4 $arg5 $arg6 $arg7"
}
demonstrate one two three four five six seven eight

# Case 2: correctly-arg-counted call to a different same-file proc (true
# negative — must stay silent)
proc need3 {a b c} {
    return "$a$b$c"
}
need3 1 2 3

# Case 3: `interp alias` partial application — the alias's arity is the
# target's arity minus the prepended argument count (true positive: only
# one argument given, two more are required)
proc target {a b c} {
    return "$a$b$c"
}
interp alias {} shortcut {} target 100
shortcut 2

# Case 4: `rename` is a pure name move — the renamed name is checked
# against the exact same arity the original proc always had (true
# positive: still requires exactly 3 arguments)
rename target target_orig
target_orig 1 2

# Case 5: TclOO method call arity, checked the same way as a proc's (true
# positive: `draw` needs exactly 2 arguments)
oo::class create Shape {
    method draw {x y} {
        return "$x,$y"
    }
}
set shape1 [Shape new]
$shape1 draw 1 2 3

# Case 6: `forward NAME my TARGET ?ARG…?` — the documented TclOO idiom for
# forwarding to a sibling method (a bare method name is never a valid
# forward target; `forward legacyDraw draw` would fail at run time with
# "invalid command name" instead). `my`'s target resolves through the
# receiver's own method-resolution order, arity-shifted by any arguments
# placed after it (true positive: `legacyDraw` takes exactly one argument,
# since `draw`'s first parameter is supplied by the forward itself)
oo::class create LegacyShape {
    method draw {x y} {
        return "$x,$y"
    }
    forward legacyDraw my draw 0
}
set shape2 [LegacyShape new]
$shape2 legacyDraw 1 2
