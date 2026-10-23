proc empty {} {}
# T3: the observable surface of a proc whose body is dispatched natively -
# recursion, arity, introspection, scope, traces, and redefinition all answer
# exactly what an interpreted body answers.
#
# Every definition this file wants bound to a compiled body is written before
# the first command that widens the world state, because a `proc` statement
# only lowers to the definition ABI while its own dispatch is still proven.
proc fib {n} {
    if {$n < 2} { return $n }
    return [expr {[fib [expr {$n - 1}]] + [fib [expr {$n - 2}]]}]
}

puts [fib 20]
