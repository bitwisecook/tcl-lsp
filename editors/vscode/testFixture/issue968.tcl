# Issue #968: the analyser never recognised a built-in `expr` math function
# (`sin`, `max`, ...) as a resolvable command — only a same-named user
# `proc ::tcl::mathfunc::<name>` override resolved. Every stock math function
# call inside `expr` drew a spurious W123 "unknown command" hint.

set builtin [expr {sin(1.0) + max(1, 2, 3)}]
puts $builtin

set nested [expr {sqrt(abs($builtin))}]
puts $nested

# Control: a genuinely unknown function name must still be flagged.
set bogus [expr {frobnicate(1)}]
puts $bogus
