# Caller-frame injection via `uplevel` (issue #923 audit idx 24 / idx 38).
#
# Verified on tclsh 9.0.4 and 8.6.16, byte-identical — the file prints
#   99
#   99
#   1 1 1
# so every variable read below really is assigned before it is read, and
# every `$varName` in a `[list set ...]` is the idiom rather than a
# name/value confusion.  No W210 and no W212 may be reported anywhere.
proc setInCaller {var} {
    uplevel 1 [list set $var 99]
}
proc useIt {} {
    setInCaller answer
    return $answer
}
puts [useIt]

proc setUp2 {var} {
    uplevel 2 [list set $var 99]
}
proc middle {} {
    setUp2 deepAnswer
}
proc outer {} {
    middle
    return $deepAnswer
}
puts [outer]

proc NewArrays {varNames value} {
    foreach varName $varNames {
        uplevel 1 [list set $varName $value]
    }
}
proc Qfrac2 {} {
    NewArrays {rdiagArray acnormArray waArray} 1
    return [list $rdiagArray $acnormArray $waArray]
}
puts [Qfrac2]

# Marker: an unused parameter draws W214, unrelated to W210/W212.  The test
# waits for it so "no W210/W212" cannot pass merely because analysis had not
# finished yet.
proc analysisRanMarker {unusedParam} {
    return done
}
