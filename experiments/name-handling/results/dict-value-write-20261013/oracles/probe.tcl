puts [list version [info patchlevel]]
if {[llength [info commands dict]] == 0} {
 puts [list dict unsupported]
} else {
 set x OLD
 set d {x NEW y Y}
 dict with d {puts [list existing-during $x $y]}
 puts [list existing-after $x $y]
 proc localDict {} {
  set d {x LOCAL}
  dict with d {puts [list proc-during $x]}
  puts [list proc-after $x]
 }
 localDict
 set x ROOT
 namespace eval N {
  set d {x NAMESPACE}
  dict with d {puts [list namespace-during $x]}
  puts [list namespace-after $x]
 }
 puts [list root-after $x]
 puts [list namespace-exists [info exists ::N::x]]
}
