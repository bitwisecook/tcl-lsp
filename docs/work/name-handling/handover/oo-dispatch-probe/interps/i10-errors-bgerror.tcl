# Errors cross the interpreter boundary with errorCode; interp bgerror (8.5+).
set c [interp create]
set rc [catch {interp eval $c {error boom {} {MY CODE}}} m opts]
puts "rc=$rc m=$m code=[dict get $opts -errorcode]"
puts "info-in-parent=[string match *boom* $::errorInfo]"
interp eval $c { proc handler {msg opts} { set ::got $msg } }
interp bgerror $c handler
puts "handler=[interp bgerror $c]"
interp eval $c { after 0 {error late} }
interp eval $c { vwait ::got }
puts "got=[interp eval $c {set ::got}]"
interp delete $c
