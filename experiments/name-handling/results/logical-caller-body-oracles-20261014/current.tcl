namespace eval N {proc outer {} {uplevel 0 {proc inner {} {set x INNER; return $x}}}}
N::outer
puts [N::inner]
puts [info procs ::inner]
puts [info procs ::N::inner]
