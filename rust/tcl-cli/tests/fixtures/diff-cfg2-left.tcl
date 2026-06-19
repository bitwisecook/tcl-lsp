proc classify {n} {
    if {$n < 0} {
        set r negative
    } elseif {$n == 0} {
        set r zero
    } else {
        set r positive
    }
    return $r
}
set total 0
foreach v {1 2 3 4} {
    set total [expr {$total + $v}]
}
set i 0
while {$i < $total} {
    incr i
}
puts "$total $i [classify $total]"
