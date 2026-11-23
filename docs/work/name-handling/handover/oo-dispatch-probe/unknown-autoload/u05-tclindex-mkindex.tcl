# auto_mkindex writes tclIndex; auto_path then autoloads the procedure on first call.
file mkdir lib
set f [open lib/a.tcl w]
puts $f {proc libproc {x} { return "libproc $x" }}
puts $f {namespace eval libns { proc helper {} { return helper } }}
close $f
auto_mkindex lib *.tcl
set f [open lib/tclIndex]; set idx [read $f]; close $f
foreach line [split $idx \n] { if {[string match "set auto_index*" $line]} { puts "idx: $line" } }
lappend auto_path [file join [pwd] lib]
puts "before: [info commands libproc]"
puts [libproc 7]
puts [libns::helper]
puts "after: [info commands libproc]"
file delete -force lib
