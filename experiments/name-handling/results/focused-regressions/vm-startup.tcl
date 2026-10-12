puts [list NH_HARNESS library [info library] tcl [package present Tcl] tcltest [package present tcltest] loader [package ifneeded tcltest [package present tcltest]]]
puts [list INIT_PRESENT [file exists [file join [info library] init.tcl]] TCLTEST_PRESENT [file exists [file join [info library] tcltest tcltest.tcl]]]
