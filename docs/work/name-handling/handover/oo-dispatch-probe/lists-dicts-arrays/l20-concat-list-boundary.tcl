set a {x {y z}}
puts [list [llength [concat $a w]] [llength [list $a w]] [llength "$a w"] [llength [list {*}$a w]]]
