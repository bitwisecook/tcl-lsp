set i 0
set sum 0
while {$i < 10} {
    incr i
    if {$i == 3} continue
    if {$i == 7} break
    incr sum
}
