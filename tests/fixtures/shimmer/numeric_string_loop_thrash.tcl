set value "hello"
for {set i 0} {$i < 3} {incr i} {
    set value [expr {$value + 1}]
    set value "v-$value"
}
