proc p {} {
    set r 1
    # comment \
    set r 2
    return $r
}
puts [p]
