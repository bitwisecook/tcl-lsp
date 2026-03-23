# List operations
set lst {a b c d e}
set len [llength $lst]
set elem [lindex $lst 2]
set sub [lrange $lst 1 3]
set sorted [lsort $lst]
set reversed [lreverse $lst]
set joined [join $lst ","]
set found [lsearch $lst "c"]
lappend lst f g
set final [llength $lst]
