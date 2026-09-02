# T2: the dict core: create/set/get/exists/incr/lappend/for/keys/size.
set d [dict create name tcl version 9]
dict set d year 2024
puts [dict get $d name]
puts [dict exists $d year]
puts [dict exists $d nope]
dict incr d version
dict lappend d tags fast
dict lappend d tags small
puts [dict get $d tags]
puts [dict size $d]
puts [lsort [dict keys $d]]
dict for {k v} $d { if {$k eq "year"} { puts "$k -> $v" } }
dict unset d year
puts [dict keys $d]
set nested [dict create a [dict create b 1]]
puts [dict get $nested a b]
dict set nested a c 2
puts [dict get $nested a]
