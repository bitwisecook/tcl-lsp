# Tcl line-continuation: \<newline><whitespace>* collapses to a single
# space, a word separator. Regression guard for the stray-backslash bug
# where _split_command_subst left the \<newline> as its own argv entry.
set r [concat fred julie \
              carol bill]
# Expect 4 elements, no stray backslash: "fred julie carol bill"
if {[llength $r] != 4} { error "llength=[llength $r], want 4, r=<$r>" }
if {[lindex $r 2] ne "carol"} { error "elem 2 = '[lindex $r 2]', want 'carol'" }
set r
