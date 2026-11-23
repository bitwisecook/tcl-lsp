# marktrusted turns a safe interp into a trusted one (parent only);
# interp children lists children; nested paths address grandchildren.
set s [interp create -safe]
puts "before=[interp issafe $s]"
interp marktrusted $s
puts "after=[interp issafe $s]"
puts "self-mark=[catch {interp eval $s {interp marktrusted {}}} m]"
interp create {outer}
interp create {outer inner}
puts "children=[lsort [interp children]]"
puts "nested=[interp eval {outer inner} {expr {1+1}}]"
puts "grand=[interp children outer]"
puts "inner-safe=[interp issafe {outer inner}]"
interp delete outer
interp delete $s
