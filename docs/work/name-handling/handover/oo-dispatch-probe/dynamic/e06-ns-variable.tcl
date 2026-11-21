namespace eval ::n { variable v 7 }
proc ::n::p {} { variable v; return $v }
proc ::n::q {} { namespace upvar ::n v w; return $w }
puts [::n::p][::n::q]
