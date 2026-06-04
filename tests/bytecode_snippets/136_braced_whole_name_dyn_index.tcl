proc test {} {
    global a
    set i 1
    set x ${a($i)}
    set y $a($i)
    set l [list ${a($i)} $a($i)]
    incr n ${a($i)}
    set outer(${a($i)}) 9
    set z $outer(${a($i)})
}
