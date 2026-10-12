set b [binary format H* 61626300]; binary scan $b a4 s
puts [list SYNTHETIC_HEX [binary encode hex $s]]
puts [list SYNTHETIC_BYTES [string bytelength $s]]
puts [list SPLIT_COUNT [llength [split $s \0]]]
