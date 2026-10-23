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
proc greet {name {greeting hello} args} { return "$greeting $name ([llength $args])" }
proc lvl {} { return [list [info level] [info level 0]] }
proc outer {x} { return [lvl] }
proc incrby {name delta} { upvar 1 $name v; incr v $delta; uplevel 1 {set seen 1} }
proc traced {x} { return [expr {$x * 2}] }
proc stepped {x} { set y $x; return [list $y $y] }
proc twice {x} { expr {$x * 2} }
proc pick {} { return first }
# The bodies whose last command leaves no interpreter result of its own: the
# procedure still answers what that command answers.
proc last {x} { set y $x }
proc bumped {x} { incr x }
proc show {x} { puts $x }
proc nothing {} { return }
proc empty {} {}
# ...and the two shapes whose answer the compiled body cannot produce, so the
# definition keeps its source body: `append` does not hand back the cell's new
# value, and a one-armed `if` completes with no result of its own.
proc grow {s} { append s ! }
proc branch {x} { if {$x} { set y yes } }


puts [fib 20]
