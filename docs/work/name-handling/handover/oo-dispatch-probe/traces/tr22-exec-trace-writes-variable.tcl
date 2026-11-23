# An execution trace on a built-in changes a global between two reads of it.
proc cb {cmd op} { set ::x 5 }
set x 99
trace add execution puts enter cb
puts "after=$x"
puts "again=$x"
