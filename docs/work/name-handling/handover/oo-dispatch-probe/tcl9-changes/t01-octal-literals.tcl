# TIP 114: 0NNN is decimal in 9.0; 0oNNN octal; 0dNNN decimal
puts "expr 010 = [expr {010 + 0}]"
puts "incr: [set n 010; incr n]"
puts "lindex: [lindex {a b c d e f g h i j} 010]"
puts "format: [format %d 010]"
puts "0o10 = [expr {0o10}]"
puts "0d10 = [catch {expr {0d10}} r] $r"
puts "08 = [catch {expr {08 + 0}} r] $r"
