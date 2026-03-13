# Nested control structures
set total 0
for {set i 0} {$i < 3} {incr i} {
    for {set j 0} {$j < 3} {incr j} {
        if {$i == $j} {
            incr total
        } else {
            incr total 2
        }
    }
}

foreach x {1 2 3} {
    foreach y {a b c} {
        append result "${x}${y} "
    }
}
