puts [list LIBRARY $::tcl_library INIT [file exists [file join $::tcl_library init.tcl]]]
puts [list TCLTEST [package require tcltest] FILE [file exists [file join $::tcl_library tcltest tcltest.tcl]]]
puts [list TESTPROC [info body ::tcltest::test]]
