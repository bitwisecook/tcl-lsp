set a [list 1 2 3]
set b [list 4 5 6]
set c [list {*}$a {*}$b]
llength $c
