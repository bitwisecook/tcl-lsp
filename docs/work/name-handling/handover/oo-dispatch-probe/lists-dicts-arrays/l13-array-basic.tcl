array set arr {b 2 a 1}
set arr(c) 3
puts [list [lsort [array names arr]] [array size arr] [array exists arr] [array exists nope] [lsort -stride 2 [array get arr]] [lsort [array names arr -glob {[ab]}]] [info exists arr(a)] [catch {set arr} m] $m]
