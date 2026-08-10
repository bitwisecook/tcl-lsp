# Issue #1332 — `source` was never followed, so this file's `winfo` drew
# W120 "requires package require Tk" even though the sourced file loads Tk.
source crossFileSourceTk.tcl
winfo exists .l
# A command nothing in the workspace defines. It is the deep-pass marker this
# file's test polls on: the progressive fast tier publishes no W120/W123 at
# all, so "W120 is absent" is only meaningful once a W12x has appeared. It is
# also a true positive in its own right — following `source` must not turn
# into a licence to go quiet.
neverDefinedAnywhereAtAll 1
