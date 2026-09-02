# T6: the dispatch features that force a full call chain: mixin, filter,
# forward, export/unexport, objdefine.
oo::class create Logged {
    filter Log
    method Log args {
        set r [next {*}$args]
        puts "[self target]: $r"
        return $r
    }
}
oo::class create Stack {
    variable items
    constructor {} { set items {} }
    method push {x} { lappend items $x; llength $items }
    method pop {} { set x [lindex $items end]; set items [lrange $items 0 end-1]; return $x }
    method size {} { llength $items }
    forward count my size
}
set s [Stack new]
$s push a
oo::objdefine $s mixin Logged
$s push b
puts [$s pop]
puts [$s count]
oo::define Stack method Peek {} { lindex $items end }
puts [catch {$s Peek} msg]
oo::define Stack export Peek
puts [$s Peek]
