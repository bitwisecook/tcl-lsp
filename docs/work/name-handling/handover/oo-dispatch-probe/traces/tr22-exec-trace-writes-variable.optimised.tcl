proc cb {cmd op} { set ::x 5 }
trace add execution puts enter cb
puts "after=99"
puts "again=99"
