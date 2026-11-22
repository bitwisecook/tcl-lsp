namespace eval ::lib { proc helper {} {return lh} ; proc use {} { helper } }
puts [list [::lib::helper] [::lib::use] [namespace eval ::lib {helper}]]
