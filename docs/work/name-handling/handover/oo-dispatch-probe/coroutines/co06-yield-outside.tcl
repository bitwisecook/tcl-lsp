# yield, yieldto and info coroutine outside any coroutine.
puts [list [catch {yield 1} m] $m]
puts [list [catch {yieldto list} m] $m]
proc p {} { yield 2 }
puts [list [catch p m] $m]
