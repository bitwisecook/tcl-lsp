package require struct::list
set v {a b c}
struct::list swap v 0 2
puts $v
struct::list delete v b
puts $v
proc odd {x} {expr {$x % 2}}
puts [struct::list split {1 2 3} odd pass fail]
puts [list $pass $fail]
set rows [struct::list dbJoin -keys keys 0 {{k A}} 0 {{k B}}]
puts [list $keys $rows]
set rows [struct::list dbJoinKeyed -keys keyed {{k A}} {{k B}}]
puts [list $keyed $rows]
