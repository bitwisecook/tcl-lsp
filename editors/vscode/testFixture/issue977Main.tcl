# Issue #977 — the sourcing file the library's own compilation unit can never
# see.  Its `issue977_helper dev` is the caller that makes the fold unsound.
source issue977Lib.tcl
issue977_helper dev
