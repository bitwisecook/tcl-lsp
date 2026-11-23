# Jim procedure statics
proc counter {} {{count 0}} {incr count}
counter
puts [counter]
