# Issue #1051 — the Tcl_ConcatObj eval family joins every trailing script word.
# Verified against tclsh 8.6.14 and tclsh 9.0.4.
eval set total 0
puts $total
eval {set label} hello
puts $label
namespace eval ::app { proc handler {tag} { puts $tag } }
namespace inscope ::app {handler} ready
# The joined script really is checked: `set` on its own is wrong # args.
eval set
