puts [list RELEASE [info patchlevel]]
set input {aâ‚}
set delimiter {}
puts SPLIT_ENTER
set r [split $input $delimiter]
puts [list COUNT [llength $r]]
foreach part $r {set code [catch {binary scan $part H* hex} detail]; if {$code} {puts [list PROJECTION_ERROR $code $detail]} else {puts [list PROJECTED_HEX $hex]}}
set r
