set rest [lassign {1 2 3 4} a b]
puts [list $a $b $rest [lmap x {1 2 3} {expr {$x*2}}] [lmap x {1 2 3} {if {$x==2} continue; set x}] [lrepeat 3 a b] [lreverse {1 2 3}] [linsert {a b} end c] [linsert {a b} 0 z]]
