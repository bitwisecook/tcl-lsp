proc add {a b} {
    return [expr {$a + $b}]
}

proc double {x} {
    return [add $x $x]
}

proc quad {x} {
    set y [double $x]
    return [double $y]
}

namespace eval ::math {
    proc triple {n} {
        return [expr {$n * 3}]
    }
}

set result [quad 5]
