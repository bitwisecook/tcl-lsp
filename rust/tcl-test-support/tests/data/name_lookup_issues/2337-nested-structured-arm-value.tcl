proc q {} {if 1 {if 1 {set a 5}}}
puts <[q]>
proc q2 {} {if 1 {switch 1 {1 {set a 5}}}}
puts <[q2]>
proc q3 {} {if 1 {eval {set a 5}}}
puts <[q3]>
