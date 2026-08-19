proc p {} {}
proc cb args {}
trace add command p {delete rename} cb
puts "cmd: [trace info command p]"
trace add execution p {leavestep leave enterstep enter} cb
puts "exec: [trace info execution p]"
trace add variable q {unset write read array} cb
puts "var: [trace info variable q]"
