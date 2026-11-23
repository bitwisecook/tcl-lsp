# An earlier store is observed by a write trace, so it is not dead.
proc log {n1 n2 op} { upvar 1 $n1 v; puts "wrote $v" }
proc p {} {
    trace add variable t write log
    set t 1
    set t 2
    return done
}
puts [p]
set k 1
trace add variable k read {log}
set k 7
puts [expr {$k + 0}]
