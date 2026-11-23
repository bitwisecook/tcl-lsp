# source with and without -encoding on a Latin-1 file; invalid UTF-8; BOM; CRLF; lone CR; ctrl-Z
set dir [pwd]
proc w {name bytes} { set f [open [file join [pwd] $name] wb]; puts -nonewline $f $bytes; close $f }
proc cps {s} { set o {}; foreach c [split $s ""] { lappend o [format %X [scan $c %c]] }; return $o }
w latin1.tcl "set v \"\xe9\"\n"
set rc [catch {source [file join $dir latin1.tcl]} m]
puts "latin1 default: $rc [expr {$rc ? $m : [cps $v]}]"
catch {unset v}
set rc [catch {source -encoding iso8859-1 [file join $dir latin1.tcl]} m]
puts "latin1 -encoding: $rc [expr {$rc ? $m : [cps $v]}]"
w bom.tcl "\xef\xbb\xbfset v ok\n"
catch {unset v}
set rc [catch {source [file join $dir bom.tcl]} m]
puts "bom: $rc [expr {$rc ? $m : $v}]"
w crlf.tcl "set v \{a\r\nb\}\r\n"
set rc [catch {source [file join $dir crlf.tcl]} m]
puts "crlf: $rc [expr {$rc ? $m : [cps $v]}]"
w cr.tcl "set v 1\rset v2 \{x\ry\}\r"
catch {unset v2}
set rc [catch {source [file join $dir cr.tcl]} m]
puts "lone cr: $rc [expr {$rc ? $m : [cps $v2]}]"
w ctrlz.tcl "set v before\n\x1aset v after\n"
set rc [catch {source [file join $dir ctrlz.tcl]} m]
puts "ctrl-z: $rc [expr {$rc ? $m : $v}]"
w utf8.tcl "set v \"\xc3\xa9\"\n"
set rc [catch {source [file join $dir utf8.tcl]} m]
puts "utf8 default: $rc [expr {$rc ? $m : [cps $v]}]"
foreach n {latin1 bom crlf cr ctrlz utf8} { file delete [file join $dir $n.tcl] }
