rename puts _puts
proc puts {args} { _puts "w:[lindex $args end]" }
puts hi
rename puts {}
rename _puts puts
puts back
