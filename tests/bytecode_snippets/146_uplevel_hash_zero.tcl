proc set_global {} {
    uplevel #0 {set ::g 42}
}
set_global
puts $::g
