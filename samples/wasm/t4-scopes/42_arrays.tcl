# T4: arrays with literal and computed keys.
array set colors {red #f00 green #0f0}
set colors(blue) #00f
puts $colors(red)
puts [array size colors]
puts [lsort [array names colors]]
set k green
puts $colors($k)
puts [info exists colors(purple)]
foreach key [lsort [array names colors]] { puts "$key=$colors($key)" }
array unset colors red
puts [lsort [array names colors]]
set m(1,2) x
puts $m(1,2)
