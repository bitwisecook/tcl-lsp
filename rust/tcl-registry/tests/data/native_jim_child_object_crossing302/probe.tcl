puts [list version [info patchlevel]]
set child [interp]
$child alias echo list
foreach {label value} [list empty {} spaced {A B} nul "A\u0000B" unicode "\u00e9\u20ac" escaped {A\B;C}] {
    set script [list list $value]
    set child_list [$child eval $script]
    set child_value [lindex $child_list 0]
    set alias_list [$child eval [list echo $value]]
    set alias_value [lindex $alias_list 0]
    puts [list roundtrip $label [string length $value] [string equal $value $child_value] [string equal $value $alias_value]]
}
puts [list concat [$child eval {list} {A B}]]
puts [list single [$child eval [list list {A B}]]]
foreach script {{return VALUE} {return -code 7 VALUE} {break} {continue} {error ERR}} {
    set code [catch {$child eval $script} value]
    puts [list completion $script $code $value]
}
namespace eval ::N {
    proc observe {} {
        set x LOCAL
        set child [interp]
        $child alias readX set x
        $child alias where namespace current
        set out [list [$child eval {readX}] [$child eval {where}]]
        $child delete
        return $out
    }
}
puts [list parent_frame [::N::observe]]
$child delete
