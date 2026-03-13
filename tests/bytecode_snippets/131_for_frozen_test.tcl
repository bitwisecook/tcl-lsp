set i 0
set j 0
for {} [expr {$i < 3}] {} {
    set j [incr i]
    if {$j > 3} break
}
