proc mysum {args} {
    set total 0
    foreach n $args {
        incr total $n
    }
    return $total
}
