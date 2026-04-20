# uplevel #0 writes to the global namespace
proc set_g {} {
    uplevel #0 {set ::g 42}
}
set_g
puts $::g
