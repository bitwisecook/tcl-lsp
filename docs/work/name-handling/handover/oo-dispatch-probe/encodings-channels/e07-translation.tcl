# output translations and input auto-detection of CRLF and lone CR
set d [file join [pwd] t07]
foreach tr {lf crlf cr binary} {
    set f [open $d w]; fconfigure $f -translation $tr; puts -nonewline $f "a\nb"; close $f
    set f [open $d r]; fconfigure $f -translation binary; set b [read $f]; close $f
    binary scan $b H* h; puts "out $tr: $h"
}
set f [open $d w]; fconfigure $f -translation binary; puts -nonewline $f "a\r\nb\rc\nd"; close $f
foreach tr {auto lf crlf cr binary} {
    set f [open $d r]; fconfigure $f -translation $tr; set s [read $f]; close $f
    set o {}; foreach c [split $s ""] { lappend o [scan $c %c] }; puts "in $tr: $o"
}
file delete $d
