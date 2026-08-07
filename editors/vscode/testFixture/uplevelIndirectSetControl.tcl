# TRUE-POSITIVE control for uplevelIndirectSet.tcl (issue #923 audit idx 24).
#
# Nothing reaches an outer frame, so `source`ing this and calling `useIt`
# aborts on tclsh 9.0.4 and 8.6.16 alike: can't read "answer": no such
# variable.  The directly-written `set $var` below is exactly the name/value
# confusion W212 exists for.  W210 must fire on line 11, W212 on line 7.
proc setLocally {var} {
    set $var 99
}
proc useIt {} {
    setLocally answer
    return $answer
}
