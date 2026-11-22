set bad1 "a \{b"
set bad2 "\{a\}b"
puts [list [llength {a {b c} "d e" f\ g}] [lindex {a\ b c} 0] [catch {llength $bad1} m] $m [catch {llength $bad2} m2] $m2 [llength ""] [llength " \t\n "]]
