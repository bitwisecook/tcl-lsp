# for whose step clause is break: terminates after one body run (W242 asks whether termination is provable).
for {set i 0} {$i < 3} {break} { set z $i }
puts "z=$z i=$i"
