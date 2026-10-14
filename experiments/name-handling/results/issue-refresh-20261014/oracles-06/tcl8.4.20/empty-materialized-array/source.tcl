eval {set {(key)} VALUE}
puts [eval {set {(key)}}]
eval {unset {(key)}}
puts [info exists {(key)}]
