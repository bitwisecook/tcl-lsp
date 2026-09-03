# T1: classic for loop, nested, with a computed bound.
set limit 4
set out {}
for {set i 0} {$i < $limit} {incr i} {
    for {set j 0} {$j <= $i} {incr j} {
        append out [expr {$i * $j}] " "
    }
}
puts [string trim $out]
