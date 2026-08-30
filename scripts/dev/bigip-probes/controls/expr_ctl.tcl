proc t {id s} { if {[catch {uplevel #0 $s} r]} { puts "$id :: ERR :: $r" } else { puts "$id :: OK :: $r" } }
t expr_adjacent_eq       {expr {"a"eq"a"}}
t expr_adjacent_cmdsub   {expr {[string length "xy"]eq"2"}}
t expr_adjacent_num      {expr {1+1}}
t expr_spaced_eq         {expr {"a" eq "a"}}
t expr_adjacent_ne       {expr {"a"ne"b"}}
t expr_adjacent_braceop  {expr {{a}eq{a}}}
