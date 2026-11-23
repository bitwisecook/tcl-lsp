# try body return inside a procedure; finally runs; on return handler.
proc p {} { try { return fromBody } finally { puts "finally ran" } }
puts "p: [p]"
proc q {} { try { return -code 2 r2 } on return {v o} { return "handled:$v:[dict get $o -code]" } }
puts "q: [q]"
proc r {} { set x [try { expr {2*3} }] ; return $x }
puts "r: [r]"
