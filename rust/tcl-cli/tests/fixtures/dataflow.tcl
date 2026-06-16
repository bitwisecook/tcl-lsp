proc add {a b} {
    return [expr {$a + $b}]
}

proc scale {x} {
    return [add $x $x]
}

proc save {v} {
    set ::store $v
}

namespace eval ::geom {
    proc area {w h} {
        return [expr {$w * $h}]
    }
}

proc report {x} {
    puts "scaled: [scale $x]"
}

set userinput [gets stdin]
eval $userinput
file delete $userinput
regexp $userinput "needle"
save $userinput
set total [scale 21]
report 21
