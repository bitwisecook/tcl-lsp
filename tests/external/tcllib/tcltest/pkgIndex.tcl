if {![package vsatisfies [package provide Tcl] 8.5-]} {return}
package ifneeded tcltest 2.5.8 [list source [file join $dir tcltest.tcl]]
