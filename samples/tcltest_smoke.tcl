package require Tcltest

testintobj set 0 42
puts [testintobj get 0]

testintobj set 1 7
testintobj mult10 1
puts [testintobj get 1]

testbooleanobj set 2 0
testbooleanobj not 2
puts [testbooleanobj get 2]
