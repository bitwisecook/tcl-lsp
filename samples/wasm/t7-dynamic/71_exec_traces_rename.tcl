# T7: execution traces and rename - command identity can change at runtime,
# so a direct call must be guarded or the plan must decline.
proc hello {} { return hi }
puts [hello]
trace add execution hello enter {apply {args {puts "enter: [lindex $args 0]"}}}
puts [hello]
rename hello hello_old
proc hello {} { return "new hi"}
puts [hello]
puts [hello_old]
