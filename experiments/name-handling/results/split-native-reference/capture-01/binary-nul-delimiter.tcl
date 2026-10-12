puts [list RELEASE [info patchlevel]]
set input [binary format H* 61626300]
set delimiter \0
puts SPLIT_ENTER
set r [split $input $delimiter]
puts [list COUNT [llength $r]]
foreach part $r {binary scan $part H* hex; puts [list PROJECTED_HEX $hex]}
set r
