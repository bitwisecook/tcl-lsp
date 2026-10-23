rename expr core_expr
proc expr args {return SHADOW}
proc demo {} {
    set value [expr {1 + 2}]
    return [expr {40 + 2}]
}
puts [demo]
puts [expr {7 * 8}]
