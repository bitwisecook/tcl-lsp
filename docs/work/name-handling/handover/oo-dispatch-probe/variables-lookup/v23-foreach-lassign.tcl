proc p {} { foreach {i j} {1 2} {}; lassign {3 4} k l; return $i$j$k$l ;# READ
}
puts [p]
