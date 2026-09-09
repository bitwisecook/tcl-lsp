proc BOOM {n1 n2 op} {error BOOM}
array set E {x X}
trace add variable E(x) read BOOM

set scripts {
    {array get E; list plain}
    {array get E; if 1 {list inside}}
    {array get E; if 0 {list no}}
    {array get E; while 0 {}}
    {array get E; for {set i 0} {$i < 0} {incr i} {list inside}}
    {array get E; foreach x {1} {list inside}}
    {array get E; lmap x {1} {list inside}}
    {array get E; switch x x {list inside}}
    {array get E; try {list inside}}
    {array get E; catch {list inside}}
    {array get E; eval {list inside}}
    {array get E; namespace eval :: {list inside}}
    {array get E; apply {{} {list inside}}}
    {array get E; dict for {k v} {a b} {list inside}}
    {array get E; time {list inside} 1}
}
set out {}
foreach script $scripts {
    set code [catch $script message options]
    set has [dict exists $options -errorcode]
    set errorcode [expr {$has ? [dict get $options -errorcode] : {}}]
    lappend out [list $code $has $errorcode]
}
