proc fact {n} {
    if {$n <= 1} {
        return 1
    }
    return [expr {$n * [fact [expr {$n - 1}]]}]
}
