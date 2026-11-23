# TIP 508: array default changes incr/append/lappend on missing keys
puts [catch {array default set a 10} r]:$r
puts [catch {incr a(x)} r]:$r
puts [catch {lappend a(y) z} r]:$r
puts [catch {array default get a} r]:$r
puts [catch {info exists a(q)} r]:$r
