# Stub only iRule-specific commands; leave `unknown` intact so builtin
# misuse (else, while arity) surfaces as a real error.
set f [lindex $argv 0]
set fh [open $f r]; set body [read $fh]; close $fh
proc log {args} {}
proc probe_body {} {}
if {[catch {proc probe_body {} $body} e]} { puts "FAIL~~define: $e"; exit 0 }
if {[catch {probe_body} e]} { puts "FAIL~~$e" } else { puts "OK~~" }
