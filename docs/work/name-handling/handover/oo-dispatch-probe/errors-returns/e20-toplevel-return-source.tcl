# return at the top level of a sourced file ends that file; return -code error from it is an error in source.
set f [open sub.tcl w]; puts $f {set ::a 1; return fromSub; set ::a 2}; close $f
puts "source: [source sub.tcl] a=$::a"
set f [open sub2.tcl w]; puts $f {return -code error -errorcode {SUB ERR} subfail}; close $f
puts "source2: [catch {source sub2.tcl} m] $m $::errorCode"
set f [open sub3.tcl w]; puts $f {break}; close $f
puts "source3: [catch {source sub3.tcl} m] [list $m]"
file delete sub.tcl sub2.tcl sub3.tcl
