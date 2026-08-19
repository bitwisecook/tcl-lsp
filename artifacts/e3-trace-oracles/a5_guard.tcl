# A5: isolate the re-entrancy guard from any ordering question.
set ::log {}
proc W3 {n1 n2 op} { lappend ::log "${n1}($n2)"; if {$n2 ne "other"} { set ::c(other) x } }
array set c {}
trace add variable c write W3
set c(k) 1
puts "guard-array: $::log"

set ::log {}
proc W4 {n1 n2 op} { lappend ::log "${n1}($n2)"; set ::d(k) 2 }
array set d {}
trace add variable d write W4
set d(k) 1
puts "guard-same-elem: $::log / d(k)=$d(k)"

set ::log {}
proc S1 args { lappend ::log s1; set ::e2 1 }
proc S2 args { lappend ::log s2 }
trace add variable e1 write S1
trace add variable e2 write S2
set e1 1
puts "guard-scalar: $::log"

set ::log {}
proc E5 {n1 n2 op} { lappend ::log "E($n2)"; if {$n2 eq "k"} { set ::f(other) 1 } }
array set f {}
trace add variable f(k) write E5
trace add variable f(other) write E5
set f(k) 1
puts "guard-elem-to-elem: $::log"
