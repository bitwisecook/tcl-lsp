proc classify {n} {
    if {$n <= 0} {
        set r nonpositive
    } else {
        set r positive
    }
    return $r
}
set total 0
foreach v {1 2 3 4 5} {
    set total [expr {$total + $v}]
}
set i 0
while {$i < $total} {
    incr i 2
}
puts "$total $i [classify $total]"
