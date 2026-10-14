proc P {} {rename eval {}; error fail}
set c [catch {eval {P}} result options]
puts [list $c $result [dict get $options -errorinfo]]
