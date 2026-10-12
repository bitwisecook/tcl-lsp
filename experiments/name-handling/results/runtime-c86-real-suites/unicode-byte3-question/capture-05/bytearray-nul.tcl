set s [binary format H* 61626300]
puts [list SPLIT_COUNT [llength [split $s \0]]]
