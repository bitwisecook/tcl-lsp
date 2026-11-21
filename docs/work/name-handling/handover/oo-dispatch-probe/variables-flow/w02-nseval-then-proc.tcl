namespace eval ::n { variable v 1 }
proc ::n::p {} { variable v; return $v }
puts [::n::p]
