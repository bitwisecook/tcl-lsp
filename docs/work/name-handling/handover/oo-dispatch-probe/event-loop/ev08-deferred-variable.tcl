# Braced callback reads a global set after registration; list-quoted callback captures the value.
proc p {} { set x captured; after 0 [list set ::cap $x]; after 0 {set ::err [catch {set x} m]; set ::msg $m} }
p
set g late-global
after 0 {set ::seen $g}
after 10 {set ::done 1}
vwait done
puts [list $cap $err $msg $seen]
