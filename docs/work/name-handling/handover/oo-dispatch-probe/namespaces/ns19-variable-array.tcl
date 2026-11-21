namespace eval ::n { variable arr; array set arr {a 1} }
proc ::n::p {} { variable arr; return $arr(a) }
puts [::n::p]
puts [catch {namespace eval ::n {variable a b c}} m]:$m
