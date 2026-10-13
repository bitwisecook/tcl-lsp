set reads 0
proc body_value {} {incr ::reads; return {return MATERIALISED}}
proc p {} [body_value]
puts [list [p] $reads [string equal [info body p] {return MATERIALISED}]]
