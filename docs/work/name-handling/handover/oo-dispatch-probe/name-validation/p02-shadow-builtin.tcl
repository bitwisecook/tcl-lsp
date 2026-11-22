namespace eval ::a {
    proc lindex {args} { return mine }
    proc p {} { return [lindex {x y} 0] }
}
puts [::a::p]
