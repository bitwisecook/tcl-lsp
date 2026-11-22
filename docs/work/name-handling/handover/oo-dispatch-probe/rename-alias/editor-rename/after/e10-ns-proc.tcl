namespace eval ::lib { proc zz {} {return lh} ; proc use {} { zz } }
puts [list [::lib::zz] [::lib::use] [namespace eval ::lib {zz}]]
