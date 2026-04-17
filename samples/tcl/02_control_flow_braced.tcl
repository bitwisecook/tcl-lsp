# 02: Control flow with braced expressions (recommended style)

set total 0

for {set i 0} {$i < 5} {incr i} {
    set total [expr {$total + $i}]
}

if {$total > 5} {
    puts "total is $total"
} else {
    puts "small total"
}
