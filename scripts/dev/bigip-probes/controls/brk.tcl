proc t {id s} { if {[catch {uplevel #0 $s} r]} { puts "$id :: ERR :: $r" } else { puts "$id :: OK :: <$r>" } }
t expr_brk   "set out \[expr\n{1+1}\]"
t lindex_brk "set out \[lindex\n{a b} 1\]"
t set_brk    "set out \[set\nq 5\]"
t while_nl   "set n 1\nwhile {\$n < 30}\n{\n set n 31\n}\nset out \$n"
