puts [list RELEASE [info patchlevel]]
set input {abc}
set delimiter {ÿ}
puts SPLIT_ENTER
set r [split $input $delimiter]
puts [list COUNT [llength $r]]
foreach part $r {binary scan $part H* hex; puts [list PROJECTED_HEX $hex]}
set r
