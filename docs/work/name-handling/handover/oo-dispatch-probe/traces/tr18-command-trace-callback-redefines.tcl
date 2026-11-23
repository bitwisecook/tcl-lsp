# An enter trace that redefines the command: the call being traced already runs the new body (tclsh prints new, new).
proc f {} { return old }
proc swap {cmd op} { proc f {} { return new } }
trace add execution f enter swap
puts [f]
puts [f]
