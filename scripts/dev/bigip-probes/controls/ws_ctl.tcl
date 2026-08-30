set v XX
proc t {id script} { if {[catch {uplevel #0 $script} r]} { puts "$id :: ERR :: $r" } else { puts "$id :: OK :: n=[llength $r] v=<$r>" } }
t brace_brace {list {a}{b}}
t brace_bare {list {a}b}
t brace_bare_brace {list {a}bc{d}}
t brace_quote {list {a}"b"}
t brace_dollar {list {a}$v}
t brace_cmdsub {list {a}[list b]}
t brace_x3 {list {a}{b}{c}}
t brace_empty {list {}{}}
t brace_multiword {list {a b}{c d}}
t brace_nested {list {a{b}}{c}}
t quote_brace {list "a"{b}}
t quote_bare {list "a"b}
t quote_quote {list "a""b"}
t quote_dollar {list "a"$v}
t bare_brace {list a{b}}
t bare_brace_tail {list a{b}c}
t cmdsub_brace {list [list a]{b}}
t cmdsub_bare {list [list a]b}
t dollar_brace {list $v{b}}
t dollar_quote {list $v"b"}
t subst_proof {list {$v}$v}
t subst_proof2 {list {$v}{$v}}
