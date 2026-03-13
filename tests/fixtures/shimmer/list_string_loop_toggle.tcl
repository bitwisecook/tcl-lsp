set bucket "one two"
for {set i 0} {$i < 3} {incr i} {
    set n [llength $bucket]
    set bucket "item-$n"
}
