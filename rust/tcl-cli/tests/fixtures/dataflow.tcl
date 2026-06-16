proc add {a b} {
    return [expr {$a + $b}]
}

proc scale {x} {
    return [add $x $x]
}

namespace eval ::geom {
    proc area {w h} {
        return [expr {$w * $h}]
    }
}

set total [scale 21]
