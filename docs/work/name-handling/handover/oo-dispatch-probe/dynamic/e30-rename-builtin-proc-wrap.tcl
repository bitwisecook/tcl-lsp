rename puts _puts
proc puts {args} { _puts "wrapped: [lindex $args end]" }
puts hello
puts stdout again
