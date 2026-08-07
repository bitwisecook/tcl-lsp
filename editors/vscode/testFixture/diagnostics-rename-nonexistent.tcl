# `rename OLD NEW` requires OLD to exist (issue #923 audit idx 5).
#
# tclsh 9.0.4 and 8.6.16 both abort this file at line 8 with
#   can't rename "definitelyNotDefinedAnywhere": command doesn't exist
# and, with that line removed, at line 9 with
#   can't delete "totallyBogusCommand": command doesn't exist
#
# Line numbers are load-bearing for diagnostics.test.ts.
rename definitelyNotDefinedAnywhere someAlias
rename totallyBogusCommand {}

proc greet {} { return hi }
rename greet hail
