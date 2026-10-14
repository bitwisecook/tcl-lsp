set {} SCALAR
puts [list scalar [set {}]]
unset {}
set {(key)} ARRAY
puts [list array [set {(key)}] [array exists {}]]
