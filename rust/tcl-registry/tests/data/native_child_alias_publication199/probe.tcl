puts [list version [info patchlevel]]
set is_jim [string match 0.* [info patchlevel]]
if {$is_jim} {set child [interp]} else {set child [interp create]}
foreach name {::A::a A::relative plain ::::extra} {
    if {$is_jim} {
        set code [catch {$child alias $name list VALUE} result]
        set call_code [catch {$child eval [list $name]} call_result]
    } else {
        set code [catch {interp alias $child $name {} list VALUE} result]
        set call_code [catch {interp eval $child [list $name]} call_result]
    }
    puts [list child_alias $name $code $result $call_code $call_result]
}
if {$is_jim} {$child delete} else {interp delete $child}
