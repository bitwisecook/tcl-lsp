# T1: while with a counter and an accumulator; break and continue inside.
set i 0
set sum 0
while {$i < 20} {
    incr i
    if {$i % 3 == 0} continue
    if {$i > 15} break
    incr sum $i
}
puts "$i $sum"
