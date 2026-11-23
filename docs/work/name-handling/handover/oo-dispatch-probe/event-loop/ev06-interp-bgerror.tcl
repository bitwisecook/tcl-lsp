# interp bgerror: handler gets the message and options dictionary; namespace-qualified handler.
namespace eval ns {
    proc h {msg opts} { set ::bg [list $msg [dict get $opts -code] [info level] [namespace current]]; set ::done 1 }
}
interp bgerror {} ns::h
puts [interp bgerror {}]
after 0 {error boom2}
after 200 {set ::done timeout}
vwait done
puts $bg
