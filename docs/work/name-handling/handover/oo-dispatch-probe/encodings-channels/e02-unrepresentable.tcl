# converting a character the target cannot represent
proc hex {b} { binary scan $b H* h; return $h }
puts [list ascii [catch {encoding convertto ascii "a\u00e9b"} r] [expr {[string length $r] < 10 ? [hex $r] : $r}]]
puts [list iso8859-1 [catch {encoding convertto iso8859-1 "a\u0100b"} r] [expr {[string length $r] < 10 ? [hex $r] : $r}]]
puts [list ascii-from [catch {encoding convertfrom ascii "a\xe9b"} r] $r [string length $r]]
