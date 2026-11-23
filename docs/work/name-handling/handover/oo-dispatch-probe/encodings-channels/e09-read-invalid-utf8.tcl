# reading a file holding invalid UTF-8 through a utf-8 channel
set d [file join [pwd] t09]
set f [open $d wb]; puts -nonewline $f "ab\xffcd"; close $f
set f [open $d r]; fconfigure $f -encoding utf-8
set rc [catch {read $f} r]
catch {close $f}
if {$rc} { puts "default: error: $r" } else {
    set o {}; foreach c [split $r ""] { lappend o [format %X [scan $c %c]] }; puts "default: $o"
}
if {![catch {encoding profiles}]} {
    foreach prof {tcl8 replace strict} {
        set f [open $d r]; fconfigure $f -encoding utf-8 -profile $prof
        set rc [catch {read $f} r]; catch {close $f}
        if {$rc} { puts "$prof: error: $r" } else {
            set o {}; foreach c [split $r ""] { lappend o [format %X [scan $c %c]] }; puts "$prof: $o"
        }
    }
}
file delete $d
