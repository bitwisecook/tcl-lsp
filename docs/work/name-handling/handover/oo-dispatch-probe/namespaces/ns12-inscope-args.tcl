namespace eval ::n { proc show args { return $args } }
puts [namespace inscope ::n show {a b} c]
puts [namespace code {show 1}]
puts [eval [namespace eval ::n {namespace code {show 1}}] {2 3}]
