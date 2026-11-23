# A background error goes to a global bgerror procedure, which runs at level 1.
proc bgerror {msg} { set ::bg [list $msg [info level] [namespace current]]; set ::done 1 }
after 0 {error boom}
after 200 {set ::done timeout}
vwait done
puts $bg
