set f [open x w]; puts -nonewline $f "\u00e9\u00e9z"; close $f
set f [open x r]; puts [list [read $f 2]]; close $f
set f [open x r]; fconfigure $f -encoding iso8859-1; puts [string length [read $f]]; close $f
set f [open x r]; fconfigure $f -translation binary; puts [string length [read $f]]; close $f
set f [open x w]; puts -nonewline $f "a\r\nb\r\n"; close $f
set f [open x r]; gets $f l; puts [string length $l]; close $f
