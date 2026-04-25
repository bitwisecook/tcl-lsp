set d [dict create a [dict create x 1 y 2] b [dict create x 3 y 4]]
dict get $d a x
dict set d a z 5
dict unset d b y
