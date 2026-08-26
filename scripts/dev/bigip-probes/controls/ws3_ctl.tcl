set v XX
proc t {id s} { if {[catch {uplevel #0 $s} r]} { puts "$id :: ERR :: $r" } else { puts "$id :: OK :: n=[llength $r] v=<$r>" } }
t dollarbrace_bare {list ${v}b}
t dollarbrace_at {list @${v}c}
t dollarbrace_x2 {list ${v}${v}}
t dollarbrace_brace {list ${v}{b}}
t dollarbrace_quote {list ${v}"b"}
