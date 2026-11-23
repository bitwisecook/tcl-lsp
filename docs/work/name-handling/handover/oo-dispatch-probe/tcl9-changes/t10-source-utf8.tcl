# 9.0: source defaults to -encoding utf-8
set f [open sub.tcl wb]
puts -nonewline $f "set v \"\xC3\xA9\"\n"
close $f
source sub.tcl
puts "len [string length $v]"
