proc area {w h} {
    return [expr {$w * $h}]
}
for {set i 0} {$i < 3} {incr i} {
    puts $i
}
