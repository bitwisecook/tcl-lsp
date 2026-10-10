# ASCII source; list/default spelling and every literal backslash are intentional.
# Question: which argv counts do original proc/apply formal lists accept under
# the selected provider, and does an original alias/rename preserve that shape?
puts "VERSION|[info patchlevel]"
proc r2286_count_record {label script} {
    set status [catch {uplevel 1 $script} result]
    set resultHex {}
    binary scan $result H* resultHex
    if {$status == 0} {set code {}} else {set code $::errorCode}
    puts "ROW|$label|$status|[string length $result]|$resultHex|$code"
}
proc r2286_count_case {label parameters} {
    set status [catch {proc r2286_count_target $parameters {return accepted}} result]
    set resultHex {}
    binary scan $result H* resultHex
    if {$status == 0} {set code {}} else {set code $::errorCode}
    puts "ROW|$label-definition|$status|[string length $result]|$resultHex|$code"
    if {$status != 0} {return}
    for {set count 0} {$count <= 5} {incr count} {
        set words [list r2286_count_target]
        for {set index 0} {$index < $count} {incr index} {lappend words "v$index"}
        r2286_count_record "$label-count-$count" $words
    }
    rename r2286_count_target {}
}
r2286_count_case empty {}
r2286_count_case required-after-default {a {b 2} c}
r2286_count_case middle-args {a args b}
r2286_count_case middle-rest-name {a {args tail} b}
r2286_count_case trailing-rest {a args}
r2286_count_case defaults-around-rest {a {b 2} args {c 3}}
r2286_count_case all-optional {{a 1} {b 2}}
r2286_count_case malformed-formal {{a b c}}
r2286_count_record apply-short {apply {{a b} {return accepted}} 1}
r2286_count_record apply-exact {apply {{a b} {return accepted}} 1 2}
r2286_count_record apply-surplus {apply {{a} {return accepted}} 1 2}
r2286_count_record apply-required-after-default {apply {{a {b 2} c} {return accepted}} 1 2}
r2286_count_record apply-middle-rest {apply {{a args b} {return accepted}} 1 2}
r2286_count_record apply-root-separators {::::apply {{a b} {return accepted}} 1}
r2286_count_record apply-alias-creation {interp alias {} r2286_count_alias {} apply}
r2286_count_record apply-alias-short {r2286_count_alias {{a b} {return accepted}} 1}
r2286_count_record apply-captured-lambda-alias-creation {interp alias {} r2286_count_captured {} apply {{a b} {return accepted}}}
r2286_count_record apply-captured-lambda-short {r2286_count_captured 1}
r2286_count_record apply-rename {rename apply r2286_count_apply}
r2286_count_record apply-renamed-short {r2286_count_apply {{a b} {return accepted}} 1}
