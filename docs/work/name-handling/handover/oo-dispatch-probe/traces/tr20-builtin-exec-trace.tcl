# Execution traces on a built-in command.
proc en {cmd op} { puts "enter {$cmd}" }
trace add execution string enter en
puts [string length abc]
trace remove execution string enter en
puts [string length abcd]
