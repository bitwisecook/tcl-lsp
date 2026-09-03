# T2: regexp/regsub with captures - runtime intrinsic, argv already built.
set line "key=value; other=42"
if {[regexp {(\w+)=(\w+)} $line -> k v]} { puts "$k $v" }
puts [regsub -all {\d} $line X]
puts [regexp -all -inline {\w+=} $line]
