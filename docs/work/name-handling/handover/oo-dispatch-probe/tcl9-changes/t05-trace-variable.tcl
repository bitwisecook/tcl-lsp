# 9.0 removed trace variable/vdelete/vinfo
proc cb {args} {puts "cb $args"}
puts [catch {trace variable x w cb} r]:$r
puts [catch {trace vinfo x} r]:$r
trace add variable y write cb
set y 1
