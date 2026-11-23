# socket server on 127.0.0.1 with an ephemeral port
proc t {label script} {if {[catch {uplevel 1 $script} r]} {puts "$label: ERR $r"} else {puts "$label: $r"}}
proc accept {s a p} { set ::srv $s; puts $s "hi"; flush $s }
t server {set ss [socket -server accept -myaddr 127.0.0.1 0]; string match sock* $ss}
t port {set port [lindex [fconfigure $ss -sockname] 2]; string is integer $port}
t client {set c [socket 127.0.0.1 $port]; string match sock* $c}
t keys {set r {}; foreach {k v} [fconfigure $c] {lappend r $k}; set r}
t gets {vwait ::srv; gets $c}
t peer {llength [fconfigure $c -peername]}
close $c; close $::srv; close $ss
