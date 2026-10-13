set reads 0
proc body_value {} {incr ::reads; return UNEXPECTED}
proc p {} {return {[body_value]}}
puts [list [string equal [p] {[body_value]}] $reads [string equal [info body p] {return {[body_value]}}]]
