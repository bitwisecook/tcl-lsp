proc p {c} {
    if {$c} { set x 1 } else { global x; set x 2 }
}
p 1
puts [info exists ::x]
