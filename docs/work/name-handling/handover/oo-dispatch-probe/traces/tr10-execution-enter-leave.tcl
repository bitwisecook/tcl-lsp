# Execution traces: enter and leave arguments, including errors.
proc en {cmd op} { puts "enter {$cmd} $op" }
proc lv {cmd code res op} { puts "leave {$cmd} $code {$res} $op" }
proc f {a} { if {$a eq "bad"} { error oops } ; return [string toupper $a] }
trace add execution f enter en
trace add execution f leave lv
puts [f x]
puts [catch {f bad} m]:$m
puts [trace info execution f]
