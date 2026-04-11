# Instance variable persistence across method calls
oo::class create Counter {
    variable count
    constructor {} { set count 0 }
    method incr {} { set count [expr {$count + 1}]; return $count }
    method get {} { return $count }
}
set c1 [Counter new]
set c2 [Counter new]
$c1 incr
$c1 incr
$c2 incr
set result [list [$c1 get] [$c2 get]]
set result
