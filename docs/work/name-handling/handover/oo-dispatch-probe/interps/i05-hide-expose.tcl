# hide/expose/invokehidden in a normal (trusted) interpreter
set c [interp create]
interp eval $c { proc secretProc {a} { return "secret:$a" } }
interp hide $c secretProc
puts "hidden=[interp hidden $c]"
puts "call=[catch {interp eval $c secretProc 1} m] $m"
puts "invokehidden=[interp invokehidden $c secretProc 2]"
interp eval $c { namespace eval ns { variable v nsvar } }
puts "global=[interp invokehidden $c -global secretProc 3]"
interp expose $c secretProc renamedProc
puts "exposed=[interp eval $c renamedProc 4]"
puts "hidden-after=[interp hidden $c]"
# hidden commands of the parent itself
proc mine {} { return mine }
interp hide {} mine
puts "parent-call=[catch {mine} m] $m"
puts "parent-hidden=[interp invokehidden {} mine]"
interp expose {} mine
puts "parent-after=[mine]"
interp delete $c
