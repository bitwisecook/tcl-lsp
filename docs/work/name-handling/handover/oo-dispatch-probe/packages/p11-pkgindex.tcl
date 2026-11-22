lappend auto_path [list <fixtures>/lib]
puts [list [package require foo] [::foo::hi] [lsort [package versions foo]]]
