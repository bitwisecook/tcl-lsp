namespace eval a::b {}
puts [list [namespace exists ::a] [namespace exists ::a::b] [catch {proc ::x::y {} {}} m] $m [catch {set ::zz::v 1} m2] $m2]
