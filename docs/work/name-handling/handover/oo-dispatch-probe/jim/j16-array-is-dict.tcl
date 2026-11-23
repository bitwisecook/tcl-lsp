# Jim arrays are dict-valued scalars
array set a {x 1 y 2}
puts [catch {dict get $a x} r]:$r
puts [catch {llength $a} r]:$r
