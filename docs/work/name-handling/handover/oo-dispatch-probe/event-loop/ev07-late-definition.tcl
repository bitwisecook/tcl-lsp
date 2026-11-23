# The callback's command is resolved when it runs: defined after registration, then redefined.
after 0 late
proc late {} { lappend ::seen first }
after 0 {rename late {}; proc late {} { lappend ::seen second }; late}
after 10 {set ::done 1}
vwait done
puts $seen
