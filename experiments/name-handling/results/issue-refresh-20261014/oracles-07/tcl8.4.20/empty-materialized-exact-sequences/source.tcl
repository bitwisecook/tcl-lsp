eval {set {} VALUE; set {}; unset {}}
puts [info exists {}]
eval {set {(key)} VALUE; set {(key)}; unset {(key)}}
puts [info exists {(key)}]
