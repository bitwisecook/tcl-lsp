oo::class create C {}; C create O
puts [list BEFORE [lsort [info class methods C -all]] [lsort [info object methods O -all]]]
oo::define ::oo::object deletemethod destroy
puts [list REMOVED [lsort [info class methods C -all]] [lsort [info object methods O -all]] [catch {O destroy} r] $r]
