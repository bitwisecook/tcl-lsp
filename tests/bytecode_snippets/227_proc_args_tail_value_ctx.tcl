# ``proc p {args} {...}`` must receive surplus call-site arguments as
# a list in its ``args`` variable — also when the proc is invoked from
# a value context (``[p ...]``), not just from statement context.
# Regression for the value-context dispatcher ignoring the args-tail
# packing path and passing only the first arg.
proc p {args} { return $args }
if {[p] ne ""} { error "p() != empty" }
if {[p hello] ne "hello"} { error "p(1 arg) != hello" }
if {[p 1 2 3] ne "1 2 3"} { error "p(3 args) = '[p 1 2 3]' (should be '1 2 3')" }

proc q {a args} { return "$a:$args" }
if {[q x] ne "x:"} { error "q(1 fixed) = '[q x]'" }
if {[q x y z] ne "x:y z"} { error "q(1 fixed + 2 tail) = '[q x y z]'" }
return ok
