# T3: recursion with a frame-observable body (no upvar) - native call, no
# Tcl frame needed unless something introspects it.
proc fib {n} {
    if {$n < 2} { return $n }
    return [expr {[fib [expr {$n - 1}]] + [fib [expr {$n - 2}]]}]
}
proc fact {n} { if {$n <= 1} { return 1 }; expr {$n * [fact [expr {$n - 1}]]} }
puts [fib 20]
puts [fact 20]
