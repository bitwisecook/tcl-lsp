set x 10
apply {{} {
    upvar 1 x local
    incr local 5
}}
