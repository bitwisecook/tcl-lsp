# Issue #977 — the sourcing file whose `dev` call the library alone can never
# see.
source issue977Lib.tcl
issue977_helper dev
