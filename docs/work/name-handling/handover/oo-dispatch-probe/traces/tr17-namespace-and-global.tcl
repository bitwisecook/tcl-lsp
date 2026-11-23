# Traces on namespace variables and on globals reached through 'global'.
proc cb {n1 n2 op} { puts "cb {$n1} {$n2} $op [namespace current]" }
namespace eval ns { variable v 0 }
trace add variable ns::v write cb
namespace eval ns { set v 1 }
set ::ns::v 2
set g 0
trace add variable ::g write cb
proc p {} { global g; set g 5 }
p
