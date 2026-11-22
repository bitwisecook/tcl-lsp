set r {}
lappend r a ;# trailing comment
# a comment with an { unbalanced brace
lappend r b
set x #notacomment
lappend r $x
puts [list $r]
