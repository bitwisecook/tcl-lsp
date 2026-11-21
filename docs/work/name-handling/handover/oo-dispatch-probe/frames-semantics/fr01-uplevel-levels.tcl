proc a {} { b; return $x }
proc b {} { c; uplevel 1 {set y fromb} }
proc c {} { uplevel 2 {set x fromc}; uplevel #0 {set ::g fromc} }
puts [a]:$::g
proc d {} { uplevel {set z dflt} ; return }
proc e {} { d; return $z }
puts [e]
proc f {} { uplevel #1 {set w hash1} }
proc g1 {} { h1; return $w }
proc h1 {} { f }
puts [g1]
