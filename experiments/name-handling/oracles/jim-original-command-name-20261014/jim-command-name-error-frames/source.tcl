proc P {n} {if {$n} {rename P Q; Q 0; error fail}; return ok}
set c [catch {P 1} result options]
puts [list code $c result $result]
puts [dict get $options -errorinfo]
