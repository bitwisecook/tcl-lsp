# writing characters above U+00FF to a binary channel; and invalid bytes to utf-8 channel
set d [file join [pwd] t08]
set f [open $d w]; fconfigure $f -translation binary
set rc [catch {puts -nonewline $f "a\u0100b"; flush $f} m]
catch {close $f}
set f [open $d r]; fconfigure $f -translation binary; set b [read $f]; close $f
binary scan $b H* h; puts "binary write: $rc $h"
set f [open $d w]; fconfigure $f -encoding ascii
set rc [catch {puts -nonewline $f "a\u00e9b"; flush $f} m]
catch {close $f}
set f [open $d r]; fconfigure $f -translation binary; set b [read $f]; close $f
binary scan $b H* h; puts "ascii write: $rc $h"
file delete $d
