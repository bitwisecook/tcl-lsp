proc setter {} {
    uplevel 1 {set y 99}
}
proc caller {} {
    set y 0
    setter
    return $y
}
puts [caller]
