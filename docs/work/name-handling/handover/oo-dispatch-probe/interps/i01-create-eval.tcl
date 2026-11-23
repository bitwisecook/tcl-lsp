# A child interpreter is a separate name world: the parent's procs and
# variables are not visible inside it, and the child's are not visible outside.
set secret parent-value
proc parentProc {} { return parent }
set c [interp create]
puts "name=$c exists=[interp exists $c]"
puts "eval=[interp eval $c {expr {6 * 7}}]"
puts "concat=[interp eval $c set x 1 \; incr x]"
puts "var-in-child=[catch {interp eval $c {set secret}} m] $m"
puts "proc-in-child=[catch {interp eval $c parentProc} m] $m"
interp eval $c { proc childProc {} { return child } ; set childVar 5 }
puts "child-proc-in-parent=[catch {childProc} m] $m"
puts "child-var-in-parent=[info exists childVar]"
puts "child-proc-in-child=[interp eval $c childProc]"
interp delete $c
puts "after-delete=[interp exists $c]"
