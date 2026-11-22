set a(k) 1; set {a b} 2; set ::g 3; set x 4
puts [list $a(k) ${a b} $::g ${x}y [catch {set y $x::} m] $m]
