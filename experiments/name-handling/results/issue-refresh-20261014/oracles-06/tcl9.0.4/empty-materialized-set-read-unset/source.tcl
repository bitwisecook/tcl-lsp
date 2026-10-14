eval {set {} FIRST}
puts [eval {set {}}]
eval {set {} SECOND}
puts [eval {set {}}]
eval {unset {}}
puts [info exists {}]
