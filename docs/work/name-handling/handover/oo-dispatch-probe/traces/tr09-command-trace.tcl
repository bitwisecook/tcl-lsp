# Command traces on rename and delete.
proc cb {old new op} { puts "cb {$old} {$new} $op" }
proc foo {} { return foo }
trace add command foo {rename delete} cb
rename foo bar
puts [trace info command bar]
puts [bar]
rename bar ""
proc baz {} {}
trace add command baz delete cb
proc baz {} {}
puts "redefined"
