# Procedure definitions and calls
proc add {a b} {
    return [expr {$a + $b}]
}

proc factorial {n} {
    if {$n <= 1} {
        return 1
    }
    return [expr {$n * [factorial [expr {$n - 1}]]}]
}

proc greet {{name "world"}} {
    return "hello $name"
}

set sum [add 3 4]
set fact5 [factorial 5]
set msg [greet]
set msg2 [greet "tcl"]
