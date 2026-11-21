proc p {} {
    lassign {1 2} a b
    regexp {(\d+)} x42 -> n
    scan "7 8" "%d %d" c d
    foreach {e f} {5 6} break
    return $a$b$n$c$d$e$f
}
puts [p]
