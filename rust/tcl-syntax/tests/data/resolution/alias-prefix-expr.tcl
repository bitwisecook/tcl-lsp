interp alias {} prefixed {} expr 100+
puts [prefixed 23]
proc demo {} {
    set value [prefixed 1]
    return $value
}
puts [demo]
