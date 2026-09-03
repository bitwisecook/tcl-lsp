# T6: the minimal TclOO shape: class, constructor, instance vars, methods,
# my, self, destroy. A light object frame should cover all of this.
oo::class create Counter {
    variable count
    constructor {{start 0}} { set count $start }
    method incr {{by 1}} { incr count $by; return [self] }
    method get {} { return $count }
    method reset {} { my variable count; set count 0 }
}
set c [Counter new 5]
$c incr
$c incr 10
puts [$c get]
$c reset
puts [$c get]
puts [info object class $c]
puts [info object isa object $c]
$c destroy
puts [info commands $c]
