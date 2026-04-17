proc safediv {a b} {
    if {$b == 0} {
        return -code error "division by zero"
    }
    return [expr {$a / $b}]
}
