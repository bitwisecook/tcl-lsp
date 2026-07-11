proc accumulate {} {
    set x 0
    while {1} {
        set x [expr {$x + 1}]
        set x [string range $x 0 end]
    }
}

proc traced {} {
    trace add variable y write {apply {{n1 n2 op} {}}}
    set y 0
    while {1} {
        set y [expr {$y + 1}]
        set y [string range $y 0 end]
    }
}
