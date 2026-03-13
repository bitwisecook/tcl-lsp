set d [dict create x 1 y 2 z 3]
dict with d {
    incr x
    incr y 10
}
