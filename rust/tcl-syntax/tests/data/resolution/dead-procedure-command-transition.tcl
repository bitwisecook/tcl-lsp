rename set stockset
proc set {args} {return CUSTOM}
proc p {} {return; rename set {}; rename stockset set}
p
set x 1
puts $x
