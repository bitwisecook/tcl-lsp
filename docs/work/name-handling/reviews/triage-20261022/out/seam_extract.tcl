proc f {} {
    set result 1
    puts [expr {2 + 3}]
    return $result
}
puts [f]
