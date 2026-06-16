proc area {w h d} {
    return [expr {$w * $h * $d}]
}
for {set i 0} {$i < 5} {incr i} {
    puts "row $i"
}
