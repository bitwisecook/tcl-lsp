package ifneeded P 1.0 [list source [file join $dir p1.tcl]]
if {0} {
    package ifneeded P 2.0 [list source [file join $dir p2.tcl]]
}
