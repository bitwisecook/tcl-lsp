proc p {} { set u 1; unset u; return [info exists u] ;# READ
}
puts [p]
