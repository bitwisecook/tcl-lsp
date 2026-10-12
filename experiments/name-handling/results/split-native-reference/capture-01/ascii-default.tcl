puts [list RELEASE [info patchlevel]]
set input {a b	c
}
puts SPLIT_ENTER
set r [split $input]
puts [list COUNT [llength $r]]
foreach part $r {binary scan $part H* hex; puts [list PROJECTED_HEX $hex]}
set r
