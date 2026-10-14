puts [list interpreter [info patchlevel]]
proc ordinary {} {if {[catch {set x 1}]} {return FAIL}; return OK}
proc rooted {} {if {[::catch {set x 1}]} {return FAIL}; return OK}
proc quoted {} {if {["catch" {set x 1}]} {return FAIL}; return OK}
proc unrelated {} {if {[set x 0]} {return FAIL}; return OK}
foreach name {ordinary rooted quoted unrelated} {
    puts [list result $name [uplevel #0 $name]]
    if {[llength [info commands ::tcl::unsupported::disassemble]]} {
        puts [list bytecode $name [::tcl::unsupported::disassemble proc $name]]
    }
}
rename catch original_catch
proc renamed {} {if {[original_catch {set x 1}]} {return FAIL}; return OK}
proc catch args {return 0}
proc shadowed {} {if {[catch {set x 1}]} {return FAIL}; return OK}
foreach name {renamed shadowed} {
    puts [list result $name [uplevel #0 $name]]
    if {[llength [info commands ::tcl::unsupported::disassemble]]} {
        puts [list bytecode $name [::tcl::unsupported::disassemble proc $name]]
    }
}
