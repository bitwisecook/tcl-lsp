set a(k) 7
set k k
puts [list braced-combined [expr {${a(k)} + 1}]]
puts [list separate-index [expr {$a($k) + 1}]]
set {$name} 11
puts [list braced-sigil [expr {${$name} + 1}]]
set {} 13
puts [list braced-empty [expr {${} + 1}]]
set {a(b)c} 17
puts [list unmatched-paren [expr {${a(b)c} + 1}]]
