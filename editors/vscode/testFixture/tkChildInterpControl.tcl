# tcl-dialect: tcl8.6
# TRUE-POSITIVE controls for tkChildInterp.tcl (issue #923 audit idx 91):
# the per-interpreter keying narrows the Tk checks, it does not disable
# them.
#
#   * lines 13-14 pack and grid siblings of the SAME `.top`, both in the
#     parent interpreter — TK1001 must fire.
#   * line 15's `.missing.child` has a parent nothing ever created — TK1002
#     must fire.
#
# Line numbers are load-bearing for tkChildInterp.test.ts.
package require Tk
frame .top
pack .top.a
grid .top.b
frame .missing.child
