# TRUE-POSITIVE control for uplevelIndirectSet.tcl (issue #923 audit idx 24).
#
# Nothing here reaches an outer frame, so tclsh 9.0.4 / 8.6.16 both abort:
#   can't read "answer": no such variable
# and the directly-written `set $var` on line 7 is exactly the name/value
# confusion W212 exists for.  W210 must fire on line 11, W212 on line 7.
proc setLocally {var} {
    set $var 99
}
proc useIt {} {
    setLocally answer
    return $answer
}
