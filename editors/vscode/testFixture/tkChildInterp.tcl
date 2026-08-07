# tcl-dialect: tcl8.6
# Tk widget checks are per-interpreter (issue #923 audit idx 91).
#
# `interp create child` gives the child its own command table and its own
# widget hierarchy, so a `.top` created inside `child eval { … }` is a
# different, unrelated widget from the parent's `.top`.  Verified against
# tclsh 9.0.4 and 8.6.16: a command created inside `child eval` is absent
# from the parent's `info commands`, and vice versa.
#
# Two claims, one file:
#   * line 19 packs `.top.a` in the CHILD and line 21 grids `.top.b` in the
#     PARENT.  Two interpreters, two hierarchies — no TK1001 conflict.
#   * line 22's `.childOnly.deep` names a widget whose parent `.childOnly`
#     is never created anywhere — TK1002 must fire on it.
#
# Line numbers are load-bearing for tkChildInterp.test.ts.
package require Tk
interp create child
load {} Tk child
child eval { frame .top; pack .top.a }
frame .top
grid .top.b
frame .childOnly.deep
