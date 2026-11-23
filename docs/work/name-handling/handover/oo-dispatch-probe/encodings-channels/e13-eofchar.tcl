# -eofchar on input; 9.0 output eofchar
set d [file join [pwd] t13]
set f [open $d wb]; puts -nonewline $f "abc\x1adef"; close $f
set f [open $d r]; puts "default read: [list [read $f]]"; close $f
set f [open $d r]; fconfigure $f -eofchar \x1a; puts "eofchar 1a: [list [read $f]] eof=[eof $f]"; close $f
set f [open $d r]; fconfigure $f -eofchar c; puts "eofchar c: [list [read $f]]"; close $f
set f [open $d w]
puts "set out eofchar: [catch {fconfigure $f -eofchar {x y}} m] [list $m]"
close $f
file delete $d
