rename expr arithmetic
proc demo {} {
    set value [arithmetic {6 * 7}]
    return [arithmetic {$value + 1}]
}
puts [demo]
