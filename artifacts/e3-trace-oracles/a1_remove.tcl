# A1: which of several identical registrations does `trace remove` delete?
# C's FOREACH_VAR_TRACE walks head->tail (newest first) and breaks at the
# first match, so the NEWEST duplicate goes.
proc cb1 args { puts "  fired cb1" }
proc cb2 args { puts "  fired cb2" }

# T1=(w,cb1) T2=(w,cb2) T3=(w,cb1); removing "w cb1" must leave cb2 then cb1.
trace add variable v write cb1
trace add variable v write cb2
trace add variable v write cb1
puts "before: [trace info variable v]"
trace remove variable v write cb1
puts "after:  [trace info variable v]"
puts "fire:"
set v 1

# Same question for the legacy spelling, and cross-spelling.
proc c3 args {}
trace variable x w c3
trace add variable x write c3
trace variable x w c3
puts "x-before: [trace info variable x] / [trace vinfo x]"
trace remove variable x write c3
puts "x-after:  [trace info variable x] / [trace vinfo x]"
trace vdelete x w c3
puts "x-after2: [trace info variable x] / [trace vinfo x]"

# Command traces: same head->tail first-match rule.
proc p {} {}
proc d1 args { puts "  d1 [lindex $args end]" }
proc d2 args { puts "  d2 [lindex $args end]" }
trace add command p delete d1
trace add command p delete d2
trace add command p delete d1
puts "cmd-before: [trace info command p]"
trace remove command p delete d1
puts "cmd-after:  [trace info command p]"
rename p {}

# Execution traces too.
proc q {} {}
proc e1 args { puts "  e1" }
proc e2 args { puts "  e2" }
trace add execution q enter e1
trace add execution q enter e2
trace add execution q enter e1
trace remove execution q enter e1
puts "exec-after: [trace info execution q]"
q
