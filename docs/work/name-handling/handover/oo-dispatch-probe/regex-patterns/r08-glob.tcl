# glob command patterns over a temporary directory
set d [file join [pwd] gd]
file mkdir $d
foreach n {a.txt b.txt c.tcl {x y.txt} .hidden ab.txt {[b].txt}} { close [open [file join $d $n] w] }
proc g {args} { if {[catch {lsort [eval glob -directory [list $::d] -tails $args]} r]} { return "error: $r" }; return $r }
puts [g *.txt]
puts [g {{a,b}.txt}]
puts [g {[ab].txt}]
puts [g {\[b\].txt}]
puts [g {?.t*}]
puts [g .*]
puts [g -nocomplain *.none]
puts [g *.none]
puts [g -types f {a*}]
puts [g {{a,c}*}]
puts [g {a{,b}.txt}]
puts [g {*}]
file delete -force $d
