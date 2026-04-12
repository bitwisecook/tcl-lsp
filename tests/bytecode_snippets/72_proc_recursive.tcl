proc fib {n} {
    if {$n <= 1} {
        return $n
    }
    expr {[fib [expr {$n - 1}]] + [fib [expr {$n - 2}]]}
}
