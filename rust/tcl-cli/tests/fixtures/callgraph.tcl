proc inc {x} {
    return [expr {$x + 1}]
}

proc dec {x} {
    return [expr {$x - 1}]
}

namespace eval ::util {
    proc tag {s} {
        return "tag:$s"
    }
}

set n [inc 41]
set m [dec 7]
