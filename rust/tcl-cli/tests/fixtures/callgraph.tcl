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

proc report {x} {
    puts "result: [quad $x]"
}

set result [quad 5]
report 5
