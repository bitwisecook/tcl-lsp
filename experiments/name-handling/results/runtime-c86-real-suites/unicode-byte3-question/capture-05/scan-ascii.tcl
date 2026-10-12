set b [binary format H* 616263]; binary scan $b a3 s
puts [list SPLIT_COUNT [llength [split $s \0]]]
