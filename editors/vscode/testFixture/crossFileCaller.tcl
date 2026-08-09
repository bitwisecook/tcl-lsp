# Issue #1331 — the caller half.
#
# `libtest` is defined in crossFileDefLib.tcl. Go-to-definition always resolved
# it; the diagnostics path did not, so it reported W123 "Unknown command" and
# never arity-checked the call.
#
# tclsh 9.0.4: `wrong # args: should be "libtest a b c"`
libtest 1 2
