# T3: a typical helper - loops, strings, lists, one proc calling another.
proc pad {s width} {
    while {[string length $s] < $width} { append s " " }
    return $s
}
proc table {rows} {
    set out {}
    foreach row $rows {
        lassign $row a b
        lappend out "[pad $a 6]|[pad $b 4]|"
    }
    return [join $out \n]
}
puts [table {{alpha 1} {beta 22} {gamma 333}}]
