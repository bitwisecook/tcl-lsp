puts [catch {source [file join $::tcl_library msgcat msgcat.tcl]} m]:[string range $m 0 80]
puts done
