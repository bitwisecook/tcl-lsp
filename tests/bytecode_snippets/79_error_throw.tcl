proc safe_divide {a b} {
    if {$b == 0} {
        error "division by zero" "" {ARITH DIVZERO}
    }
    return [expr {$a / $b}]
}
