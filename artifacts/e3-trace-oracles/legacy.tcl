proc cb args {}
proc cb2 args {}
catch {trace zzz} e; puts "badopt: $e"
catch {trace variable x} e; puts "wa-var: $e"
catch {trace vdelete x} e; puts "wa-vdel: $e"
catch {trace vinfo x y} e; puts "wa-vinfo: $e"
catch {trace variable x q cb} e; puts "badops: $e"
catch {trace vdelete x {read write} cb} e; puts "badops2: $e"
trace variable x rrw cb
puts "vinfo: [trace vinfo x]"
puts "info: [trace info variable x]"
trace var x w cb2
puts "vinfo2: [trace vinfo x]"
trace vdelete x wr cb
puts "vinfo3: [trace vinfo x]"
trace add variable y {write read} cb
puts "vinfo-modern: [trace vinfo y]"
trace vdelete y rw cb
puts "vinfo-modern2: [trace vinfo y]"
trace variable z a cb
puts "zz: [trace vinfo z] / [trace info variable z]"
trace variable e2 rwua cb
puts "all: [trace vinfo e2] / [trace info variable e2]"
puts "exists: [info exists x]"
catch {trace v x} e; puts "amb: $e"
