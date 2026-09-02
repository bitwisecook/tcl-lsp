# T7: uplevel/eval/subst - dynamic scripts; frames must be real.
proc repeat {n body} {
    for {set i 0} {$i < $n} {incr i} { uplevel 1 $body }
}
set total 0
repeat 3 { incr total 2 }
puts $total
set cmd [list set dyn 42]
eval $cmd
puts $dyn
puts [subst {total=$total dyn=[expr {$dyn + 1}]}]
proc show {} { uplevel 1 { puts "caller sees total=$total" } }
show
puts [info level]
